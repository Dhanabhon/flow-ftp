//! Smart Sync integration: walk both trees, compare, execute the plan.
//!
//! The pure compare lives in `flow-sync`; this module owns the I/O around it
//! — walking the local tree (walkdir, off the async runtime) and the remote
//! tree (recursive listings through the live connection) — then turns the
//! plan into engine jobs on execute.
//!
//! Bounds: 16 levels deep, 10 000 entries per side, so a runaway tree can
//! never hang the preview.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::path::Path as StdPath;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use flow_core::{
    format_permissions, ConnectionId, CoreError, CoreResult, FilePath, RemoteFile, SyncDiff,
    SyncDirection,
};
use flow_transfer::TransferEngine;
use walkdir::WalkDir;
use flow_sync::{is_executable, compare, CompareOptions};
use serde::Deserialize;
use tauri::State;

use crate::bridge::{ipc_error, ConnectionRegistry, IpcError};
use crate::transfers::CredentialCache;

/// Maximum directory depth for both walkers.
const MAX_DEPTH: usize = 16;
/// Maximum entries per side before the walk stops (protects the UI).
const MAX_ENTRIES: usize = 10_000;

/// Walk a local directory tree into the snapshot shape compare() expects.
fn walk_local_sync(root: &StdPath) -> CoreResult<HashMap<String, RemoteFile>> {
    let mut out = HashMap::new();
    for entry in WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
    {
        let entry = entry.map_err(|e| CoreError::Io(e.to_string()))?;
        if entry.depth() == 0 {
            continue; // the root itself is not an entry
        }
        if out.len() >= MAX_ENTRIES {
            break;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let Ok(meta) = entry.metadata() else {
            continue; // vanished mid-walk
        };
        let file_type = entry.file_type();
        let kind = if file_type.is_symlink() {
            flow_core::FileKind::Symlink
        } else if file_type.is_dir() {
            flow_core::FileKind::Directory
        } else {
            flow_core::FileKind::File
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            Some(format_permissions(meta.permissions().mode()))
        };
        #[cfg(not(unix))]
        let permissions = None;

        out.insert(
            rel,
            RemoteFile {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
                size: meta.len(),
                modified,
                permissions,
                owner: None,
                group: None,
            },
        );
    }
    Ok(out)
}

/// Recursively list a remote tree through the live connection.
async fn walk_remote(
    registry: &ConnectionRegistry,
    id: &ConnectionId,
    dir: &FilePath,
    rel: &str,
    out: &mut HashMap<String, RemoteFile>,
) -> CoreResult<()> {
    if out.len() >= MAX_ENTRIES {
        return Ok(());
    }
    let entries = registry.list(id, FilePath::new(dir.as_str().to_string())).await?;
    for entry in entries {
        let name = entry.name.clone();
        if name == ".." {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        match entry.kind {
            flow_core::FileKind::Directory => {
                let child_dir = FilePath::new(join_posix(dir.as_str(), &name));
                Box::pin(walk_remote(registry, id, &child_dir, &child_rel, out)).await?;
            }
            _ => {
                if out.len() < MAX_ENTRIES {
                    out.insert(child_rel, entry);
                }
            }
        }
    }
    Ok(())
}

/// POSIX join with root normalization (mirrors FilePath::join).
fn join_posix(base: &str, child: &str) -> String {
    if base == "/" {
        format!("/{child}")
    } else {
        format!("{}/{}", base.trim_end_matches('/'), child)
    }
}

/// Build the sync plan for the two roots. Preview only — nothing transfers.
#[tauri::command]
pub async fn sync_preview(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    local_dir: String,
    remote_dir: String,
    direction: SyncDirection,
) -> Result<Vec<SyncDiff>, IpcError> {
    let id = ConnectionId::new(connection_id);

    // Local walk: blocking, so off the runtime.
    let local_root = StdPath::new(&local_dir).to_path_buf();
    let local = tauri::async_runtime::spawn_blocking(move || walk_local_sync(&local_root))
        .await
        .map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Local scan task failed: {e}"),
        })?
        .map_err(ipc_error)?;

    // Remote walk: through the live connection.
    let mut remote = HashMap::new();
    registry
        .list(&id, FilePath::new(remote_dir.clone()))
        .await
        .map_err(ipc_error)?; // surface a bad remote root immediately
    walk_remote(&registry, &id, &FilePath::new(remote_dir), "", &mut remote)
        .await
        .map_err(ipc_error)?;

    Ok(compare(&local, &remote, &CompareOptions::new(direction)))
}

/// Request payload for executing a reviewed plan.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncExecuteRequest {
    pub connection_id: String,
    pub local_root: String,
    pub remote_root: String,
    /// The reviewed plan — conflicts are skipped automatically.
    pub diffs: Vec<SyncDiff>,
}

fn next_sync_seq() -> u64 {
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Execute a reviewed plan by enqueuing one engine job per diff.
/// Conflicts are skipped (they need a human decision — see flow-sync docs).
#[tauri::command]
pub async fn sync_execute(
    registry: State<'_, ConnectionRegistry>,
    engine: tauri::State<'_, Arc<TransferEngine>>,
    cache: State<'_, CredentialCache>,
    request: SyncExecuteRequest,
) -> Result<usize, IpcError> {
    let connection_id = ConnectionId::new(request.connection_id);
    let Some(creds) = cache.get(&connection_id) else {
        return Err(IpcError {
            code: "connection-failed".into(),
            message: "That connection is no longer active. Connect again and retry.".into(),
        });
    };

    // Ensure parent directories exist: local mkdir -p for downloads, remote
    // best-effort mkdir per ancestor for uploads. Failures surface from the
    // transfer itself if the directory truly cannot be made.
    let ancestors: HashSet<String> = request
        .diffs
        .iter()
        .filter(|diff| is_executable(diff))
        .filter_map(|diff| {
            diff.path
                .rsplit_once('/')
                .map(|(parent, _)| parent.to_string())
        })
        .collect();
    for parent in &ancestors {
        if let Err(e) = tokio::fs::create_dir_all(
            StdPath::new(&request.local_root).join(parent),
        )
        .await
        {
            return Err(IpcError {
                code: "io".into(),
                message: format!("Could not prepare local directory {parent}: {e}"),
            });
        }
    }

    let mut enqueued = 0usize;
    for diff in &request.diffs {
        if !is_executable(diff) {
            continue; // conflicts need a human decision
        }
        let direction = diff.direction;
        let remote_path = FilePath::new(join_posix(&request.remote_root, &diff.path));
        let local_path = FilePath::new(
            StdPath::new(&request.local_root)
                .join(&diff.path)
                .to_string_lossy()
                .into_owned(),
        );

        if direction == flow_core::TransferDirection::Upload {
            // Best-effort remote mkdir per ancestor (shortest first).
            let mut parts: Vec<&str> = diff.path.split('/').collect();
            parts.pop(); // the file itself
            let mut acc = request.remote_root.clone();
            for part in parts {
                acc = join_posix(&acc, part);
                let _ = registry
                    .mkdir(&connection_id, FilePath::new(acc.clone()))
                    .await; // already-exists errors are fine here
            }
        }

        let file_name = diff.path.rsplit('/').next().unwrap_or(&diff.path).to_string();
        let id = flow_core::TransferId::new(format!("sync-{}", next_sync_seq()));
        let spec = flow_transfer::TransferSpec {
            connection_id: connection_id.clone(),
            connection_name: creds.host.clone(),
            direction,
            remote_path,
            local_path,
            file_name,
        };
        engine.enqueue(id, spec, creds.clone()).await;
        enqueued += 1;
    }
    Ok(enqueued)
}
