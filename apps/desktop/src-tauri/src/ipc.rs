//! Tauri commands — the IPC boundary between the SvelteKit frontend and the
//! Rust application layer.
//!
//! Commands are thin: validate nothing (the domain owns that), look the
//! connection up in the [`ConnectionRegistry`], delegate, and map
//! [`CoreError`] into [`IpcError`] so the frontend always gets a
//! human-readable `{ code, message }`.

use flow_core::{
    format_permissions, Connection, ConnectionId, ConnectionStatus, Credentials, FilePath,
    Protocol, RemoteFile,
};
use flow_keychain::KeychainStore;
use serde::Deserialize;
use tauri::State;

use crate::bridge::{ipc_error, ConnectionRegistry, IpcError};
use crate::transfers::CredentialCache;

/// Request payload for `remote_connect`. The frontend generates the id
/// (UUID) and owns the connection list; the backend only holds live
/// sessions.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub id: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub save_keychain: bool,
}

/// Establish a remote session. Optionally persists the password in the OS
/// keychain. Returns the connection record the UI should display.
#[tauri::command]
pub async fn remote_connect(
    registry: State<'_, ConnectionRegistry>,
    cache: State<'_, CredentialCache>,
    request: ConnectRequest,
) -> Result<Connection, IpcError> {
    let id = ConnectionId::new(request.id.clone());
    let mut creds = Credentials::new(&request.host, &request.username, request.protocol)
        .with_port(request.port);
    if !request.password.is_empty() {
        creds = creds.with_password(request.password.clone());
    }

    registry
        .connect(id.clone(), creds.clone())
        .await
        .map_err(ipc_error)?;
    // Stash for the transfer engine's own dials (in-memory only).
    cache.insert(id.clone(), creds);

    // Keychain writes are synchronous and touch the OS — off the async runtime.
    if request.save_keychain && !request.password.is_empty() {
        let secret = flow_core::SecretString::new(request.password.clone());
        let key_id = id.clone();
        let saved = tokio::task::spawn_blocking(move || {
            KeychainStore::new().save_password(&key_id, &secret)
        })
        .await
        .map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Keychain task failed: {e}"),
        })?
        .map_err(ipc_error);
        if let Err(e) = saved {
            // Connect succeeded; a keychain failure must not lose the session.
            // Log and continue — the UI already shows the connection as up.
            eprintln!("keychain save failed: {}", e.message);
        }
    }

    Ok(Connection {
        id,
        name: request.host.clone(),
        protocol: request.protocol,
        host: request.host,
        port: request.port,
        username: request.username,
        status: ConnectionStatus::Connected,
        keychain: request.save_keychain,
        favorite: false,
        last_connected: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        ),
    })
}

/// Tear down a remote session. Unknown ids succeed (stale UI state).
#[tauri::command]
pub async fn remote_disconnect(
    registry: State<'_, ConnectionRegistry>,
    cache: State<'_, CredentialCache>,
    connection_id: String,
) -> Result<(), IpcError> {
    let id = ConnectionId::new(connection_id);
    cache.remove(&id);
    registry.disconnect(&id).await.map_err(ipc_error)
}

/// List a remote directory. A synthetic `..` entry is prepended for
/// non-root paths so the UI matches Finder navigation.
#[tauri::command]
pub async fn remote_list(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    path: String,
) -> Result<Vec<RemoteFile>, IpcError> {
    let id = ConnectionId::new(connection_id);
    let dir = FilePath::new(path);
    let files = registry.list(&id, dir.clone()).await.map_err(ipc_error)?;
    Ok(with_parent_entry(files, &dir))
}

/// Stat a single remote path (powers the preview panel).
#[tauri::command]
pub async fn remote_stat(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    path: String,
) -> Result<RemoteFile, IpcError> {
    let id = ConnectionId::new(connection_id);
    registry
        .stat(&id, FilePath::new(path))
        .await
        .map_err(ipc_error)
}

/// Create a remote directory.
#[tauri::command]
pub async fn remote_mkdir(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    path: String,
) -> Result<(), IpcError> {
    let id = ConnectionId::new(connection_id);
    registry
        .mkdir(&id, FilePath::new(path))
        .await
        .map_err(ipc_error)
}

/// Rename/move a remote path.
#[tauri::command]
pub async fn remote_rename(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    from: String,
    to: String,
) -> Result<(), IpcError> {
    let id = ConnectionId::new(connection_id);
    registry
        .rename(&id, FilePath::new(from), FilePath::new(to))
        .await
        .map_err(ipc_error)
}

/// Delete a remote file or empty directory.
#[tauri::command]
pub async fn remote_delete(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    path: String,
) -> Result<(), IpcError> {
    let id = ConnectionId::new(connection_id);
    registry
        .delete(&id, FilePath::new(path))
        .await
        .map_err(ipc_error)
}

/// The user's home directory — the local pane's start location.
#[tauri::command]
pub fn local_home() -> Result<String, IpcError> {
    std::env::var("HOME")
        .map_err(|_| IpcError {
            code: "io".into(),
            message: "Home directory is not defined on this system.".into(),
        })
        .map(|h| format!("{h}/"))
}

/// List a local directory in the shared file-entry shape.
#[tauri::command]
pub async fn local_list(path: String) -> Result<Vec<RemoteFile>, IpcError> {
    let mut entries = Vec::new();
    let mut reader = tokio::fs::read_dir(&path)
        .await
        .map_err(|e| IpcError {
            code: if e.kind() == std::io::ErrorKind::NotFound {
                "not-found".into()
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                "permission".into()
            } else {
                "io".into()
            },
            message: format!("Could not read {}: {}", display_path(&path), e),
        })?;

    while let Some(entry) = reader
        .next_entry()
        .await
        .map_err(|e| IpcError { code: "io".into(), message: e.to_string() })?
    {
        let Ok(meta) = entry.metadata().await else {
            continue; // vanished between readdir and stat — skip
        };
        let file_type = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue,
        };
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
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_millis() as i64)
            })
            .unwrap_or(0);
        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            Some(format_permissions(meta.permissions().mode()))
        };
        #[cfg(not(unix))]
        let permissions = None;

        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(RemoteFile {
            name,
            kind,
            size: meta.len(),
            modified,
            permissions,
            owner: None,
            group: None,
        });
    }
    Ok(entries)
}

/// `..` parent entry prepended to non-root listings.
fn with_parent_entry(files: Vec<RemoteFile>, dir: &FilePath) -> Vec<RemoteFile> {
    if dir.is_root() {
        files
    } else {
        let mut all = vec![RemoteFile::parent_entry()];
        all.extend(files);
        all
    }
}

/// Shorten a path for error messages (the full path is in context anyway).
fn display_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
