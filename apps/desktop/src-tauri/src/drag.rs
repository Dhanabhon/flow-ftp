//! Native drag-out: drag remote files from the FlowFTP window straight to
//! Finder, the Desktop, Mail, or any drop target — via
//! [`tauri_plugin_dragout`](https://crates.io/crates/tauri-plugin-dragout).
//!
//! Items are **lazy file promises**: the drag session starts instantly (the
//! user is holding the mouse), and the file's content downloads from the
//! server only when the user actually drops — written directly into the
//! destination on a background queue, exactly how Cyberduck/Transmit do it.
//! No size limit, no temp staging, nothing downloaded for a cancelled drag.

use std::path::{Path, PathBuf};

use flow_core::FilePath;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dragout::{start_drag, DragItem, DragOptions};

use crate::bridge::{build_adapter, ipc_error, IpcError};
use crate::transfers::CredentialCache;

/// Download one remote file to `dest` over a fresh connection.
///
/// The promise writer runs on the plugin's background queue, so the async
/// work is driven through `block_on` there — the UI thread never blocks.
async fn download_to_file(
    app: AppHandle,
    connection_id: String,
    remote_path: String,
    dest: PathBuf,
) -> Result<(), String> {
    let cache = app.state::<CredentialCache>();
    let id = flow_core::ConnectionId::new(connection_id);
    let Some(creds) = cache.get(&id) else {
        return Err("This connection is no longer active. Connect again and retry.".into());
    };

    let mut adapter = build_adapter(creds.protocol);
    adapter
        .connect(&creds)
        .await
        .map_err(|e| ipc_error(e).message)?;
    let mut read = adapter
        .open_read(&FilePath::new(remote_path), 0)
        .await
        .map_err(|e| ipc_error(e).message)?;

    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| format!("Could not create {}: {e}", dest.display()))?;
    tokio::io::copy(&mut read, &mut file)
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    Ok(())
}

/// Start a native drag session carrying the given remote files.
///
/// Called from a `pointermove` handler while the button is held (the plugin
/// requires a pressed mouse button). Returns the session id, which matches
/// the `session` field of the `dragout://*` events.
#[tauri::command]
pub fn drag_out_remote(
    window: WebviewWindow,
    cache: State<'_, CredentialCache>,
    connection_id: String,
    remote_dir: String,
    names: Vec<String>,
) -> Result<u64, IpcError> {
    let id = flow_core::ConnectionId::new(connection_id.clone());
    if cache.get(&id).is_none() {
        return Err(IpcError {
            code: "connection-failed".into(),
            message: "That connection is no longer active. Connect again and retry.".into(),
        });
    }

    let app = window.app_handle().clone();
    let items: Vec<DragItem> = names
        .iter()
        .map(|name| {
            let remote_path = if remote_dir == "/" {
                format!("/{name}")
            } else {
                format!("{remote_dir}/{name}")
            };
            DragItem::promise(name.clone(), {
                let app = app.clone();
                let connection_id = connection_id.clone();
                move |dest: &Path| {
                    tauri::async_runtime::block_on(download_to_file(
                        app.clone(),
                        connection_id.clone(),
                        remote_path.clone(),
                        dest.to_path_buf(),
                    ))
                }
            })
        })
        .collect();

    start_drag(&window, items, DragOptions::default())
        .map(|session| session.id)
        .map_err(|e| IpcError {
            code: "protocol".into(),
            message: e.to_string(),
        })
}

/// Classify dropped local paths so the UI can refuse folders honestly
/// (the transfer engine moves files, not directory trees).
#[tauri::command]
pub fn local_path_kinds(paths: Vec<String>) -> Vec<PathKind> {
    paths
        .into_iter()
        .map(|path| {
            let is_dir = std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
            PathKind { path, is_dir }
        })
        .collect()
}

/// One dropped path and whether it is a directory.
#[derive(serde::Serialize)]
pub struct PathKind {
    pub path: String,
    pub is_dir: bool,
}
