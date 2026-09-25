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
use crate::edit::EditSessions;
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
    /// Load the password from the OS keychain (saved profiles) instead of
    /// taking it from the request. The secret never transits to the UI.
    #[serde(default)]
    pub use_keychain_password: bool,
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
    if request.use_keychain_password {
        let key_id = id.clone();
        let stored = tokio::task::spawn_blocking(move || {
            flow_keychain::KeychainStore::new().load_password(&key_id)
        })
        .await
        .map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Keychain task failed: {e}"),
        })?
        .map_err(ipc_error)?;
        match stored {
            Some(secret) => creds = creds.with_password(secret.reveal().to_string()),
            None => {
                return Err(IpcError {
                    code: "auth-failed".into(),
                    message: "No saved password for this connection. Connect once with a                         password (and Save to Keychain) to store it."
                        .into(),
                })
            }
        }
    } else if !request.password.is_empty() {
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
    edits: State<'_, EditSessions>,
    connection_id: String,
) -> Result<(), IpcError> {
    let id = ConnectionId::new(connection_id);
    cache.remove(&id);
    edits.remove_connection(&id);
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

/// Map a local filesystem error to the IPC error shape, with kind-aware codes.
fn local_io_error(path: &str, e: std::io::Error) -> IpcError {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "not-found",
        std::io::ErrorKind::PermissionDenied => "permission",
        std::io::ErrorKind::AlreadyExists => "protocol",
        _ => "io",
    };
    IpcError {
        code: code.into(),
        message: format!("{}: {}", display_path(path), e),
    }
}

/// Create a local directory.
#[tauri::command]
pub async fn local_mkdir(path: String) -> Result<(), IpcError> {
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(|e| local_io_error(&path, e))
}

/// Rename/move a local path.
#[tauri::command]
pub async fn local_rename(from: String, to: String) -> Result<(), IpcError> {
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|e| local_io_error(&from, e))
}

/// Delete a local file or empty directory.
#[tauri::command]
pub async fn local_delete(path: String) -> Result<(), IpcError> {
    let meta = tokio::fs::metadata(&path).await;
    match meta {
        Ok(m) if m.is_dir() => tokio::fs::remove_dir(&path).await,
        _ => tokio::fs::remove_file(&path).await,
    }
    .map_err(|e| local_io_error(&path, e))
}

/// Set the transfer engine's bandwidth budget in bytes/sec (0 = unlimited).
#[tauri::command]
pub fn transfer_set_rate_limit(
    engine: tauri::State<'_, std::sync::Arc<flow_transfer::TransferEngine>>,
    bytes_per_sec: u64,
) -> Result<(), IpcError> {
    engine.set_rate_limit(bytes_per_sec);
    Ok(())
}
