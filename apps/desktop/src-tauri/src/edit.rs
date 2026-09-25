//! Remote editing: download → open in the system editor → watch → re-upload.
//!
//! The Transmit-style loop. One [`EditSession`] per open file keeps the
//! temp-file path and the watch handle — the watcher lives inside the
//! session map (managed state, alive for the app's lifetime), so dropping a
//! session stops the watch and disconnects the channel, which ends the
//! watcher thread.
//!
//! Temp files live under `$TMPDIR/flowftp-edit/<key>/`, where `<key>` hashes
//! the connection id + remote path — reopening the same file reuses the same
//! temp location, so editor tabs and watch state stay coherent.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use flow_core::{ConnectionId, CoreResult, FilePath};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::bridge::{build_adapter, ipc_error, IpcError};
use crate::transfers::CredentialCache;

/// Quiet period after the last write event before we re-upload.
const DEBOUNCE: Duration = Duration::from_millis(750);
/// Ignore watcher noise this long after our own download writes the file.
const SELF_WRITE_GRACE: Duration = Duration::from_secs(2);

/// One open remote file being edited locally.
struct EditSession {
    connection_id: ConnectionId,
    remote_path: FilePath,
    file_name: String,
    /// Keeps the watcher alive; dropping it stops watching.
    _watcher: RecommendedWatcher,
}

/// All live edit sessions, keyed by temp file path.
#[derive(Default)]
pub struct EditSessions {
    sessions: Mutex<HashMap<PathBuf, EditSession>>,
}

impl EditSessions {
    /// Drop every session belonging to a connection: watch stops, temp files go.
    pub fn remove_connection(&self, connection_id: &ConnectionId) {
        let mut sessions = self.sessions.lock().expect("edit sessions lock poisoned");
        let stale: Vec<PathBuf> = sessions
            .iter()
            .filter(|(_, session)| &session.connection_id == connection_id)
            .map(|(path, _)| path.clone())
            .collect();
        for path in stale {
            if let Some(session) = sessions.remove(&path) {
                // Dropping the session stops the watcher.
                let _ = std::fs::remove_file(&path);
                if let Some(parent) = path.parent() {
                    let _ = std::fs::remove_dir(parent);
                }
                drop(session);
            }
        }
    }
}

/// Events the frontend toasts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum EditEvent {
    Synced { file_name: String },
    Failed { file_name: String, message: String },
}

/// Stable per-file key: connection + remote path.
fn session_key(connection_id: &str, remote_path: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    connection_id.hash(&mut hasher);
    remote_path.hash(&mut hasher);
    hasher.finish()
}

fn temp_dir_for(key: u64) -> PathBuf {
    std::env::temp_dir().join(format!("flowftp-edit/{key:x}"))
}

/// Download a remote file into its temp location and open it in the system
/// editor. Starts (or restarts) the save-watcher.
#[tauri::command]
pub async fn remote_edit_open(
    app: AppHandle,
    cache: State<'_, CredentialCache>,
    sessions: State<'_, EditSessions>,
    connection_id: String,
    remote_path: String,
) -> Result<String, IpcError> {
    let id = ConnectionId::new(connection_id);
    let remote = FilePath::new(remote_path);
    let file_name = remote.name().to_string();

    let Some(creds) = cache.get(&id) else {
        return Err(IpcError {
            code: "connection-failed".into(),
            message: "That connection is no longer active. Connect again and retry.".into(),
        });
    };

    let key = session_key(id.as_str(), remote.as_str());
    let local_dir = temp_dir_for(key);
    let local_path = local_dir.join(&file_name);

    // Fresh download over a dedicated short-lived connection (the engine
    // model): no contention with the interactive session.
    tokio::fs::create_dir_all(&local_dir)
        .await
        .map_err(|e| IpcError { code: "io".into(), message: e.to_string() })?;
    let download_started = Instant::now();
    {
        let mut adapter = build_adapter(creds.protocol);
        adapter
            .connect(&creds)
            .await
            .map_err(ipc_error)?;
        let downloaded = adapter
            .download(
                flow_core::TransferId::new("edit-open"),
                &remote,
                &FilePath::new(local_path.to_string_lossy().into_owned()),
            )
            .await;
        let _ = adapter.disconnect().await;
        downloaded.map_err(ipc_error)?;
    }

    // Open with the system default application for this file type.
    app.opener()
        .open_path(local_path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Could not open an editor: {e}"),
        })?;

    start_watcher(
        app.clone(),
        &sessions,
        id,
        remote,
        file_name,
        local_path.clone(),
        download_started,
    )
    .map_err(|e| IpcError { code: "io".into(), message: e.to_string() })?;

    Ok(local_path.to_string_lossy().into_owned())
}

/// Watch the temp file; after a quiet period following writes, re-upload.
fn start_watcher(
    app: AppHandle,
    sessions: &EditSessions,
    connection_id: ConnectionId,
    remote_path: FilePath,
    file_name: String,
    local_path: PathBuf,
    download_started: Instant,
) -> Result<(), notify::Error> {
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&local_path, RecursiveMode::NonRecursive)?;

    // Register the session first: the watcher must outlive the thread, and
    // dropping the session later disconnects the channel and ends the thread.
    sessions
        .sessions
        .lock()
        .expect("edit sessions lock poisoned")
        .insert(
            local_path.clone(),
            EditSession {
                connection_id: connection_id.clone(),
                remote_path: remote_path.clone(),
                file_name: file_name.clone(),
                _watcher: watcher,
            },
        );

    std::thread::spawn(move || {
        let grace = download_started + SELF_WRITE_GRACE;
        while let Ok(event) = rx.recv() {
            let Ok(event) = event else { continue };
            let is_write = matches!(event.kind, EventKind::Modify(_));
            if !is_write || event.paths.iter().all(|path| path != &local_path) {
                continue;
            }
            // Debounce: wait for a quiet period after the last write.
            loop {
                match rx.recv_timeout(DEBOUNCE) {
                    Ok(Ok(_)) => continue, // more writes → keep waiting
                    Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            if Instant::now() < grace {
                continue; // our own initial download echoing back, not a save
            }

            let app = app.clone();
            let local = local_path.clone();
            tauri::async_runtime::spawn(async move {
                upload_edit(app, local).await;
            });
        }
    });

    Ok(())
}

/// Upload an edited temp file back to its remote location and report.
async fn upload_edit(app: AppHandle, local_path: PathBuf) {
    let (connection_id, remote_path, file_name) = {
        let sessions = app.state::<EditSessions>();
        let guard = sessions.sessions.lock().expect("edit sessions lock poisoned");
        let Some(session) = guard.get(&local_path) else {
            return; // session dropped while we were debouncing
        };
        (
            session.connection_id.clone(),
            session.remote_path.clone(),
            session.file_name.clone(),
        )
    };

    let result: CoreResult<()> = async {
        let cache = app.state::<CredentialCache>();
        let Some(creds) = cache.get(&connection_id) else {
            return Err(flow_core::CoreError::Protocol(
                "connection is no longer active".into(),
            ));
        };
        let mut adapter = build_adapter(creds.protocol);
        adapter.connect(&creds).await?;
        let uploaded = adapter
            .upload(
                flow_core::TransferId::new("edit-save"),
                &FilePath::new(local_path.to_string_lossy().into_owned()),
                &remote_path,
            )
            .await;
        let _ = adapter.disconnect().await;
        uploaded
    }
    .await;

    let event = match result {
        Ok(()) => EditEvent::Synced { file_name },
        Err(e) => EditEvent::Failed {
            file_name,
            message: e.to_string(),
        },
    };
    let _ = app.emit("edit:update", &event);
}
