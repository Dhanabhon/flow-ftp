//! Transfer queue integration: credential cache, engine wiring, commands.
//!
//! The engine dials its own connections, so it needs credentials after the
//! interactive connect is long gone: the [`CredentialCache`] stashes them in
//! memory (never persisted) when a session is established, and the connector
//! pulls from there when an attempt starts.

use std::collections::HashMap;
use std::sync::Arc;

use flow_core::{ConnectionId, CoreResult, Credentials, FilePath, TransferDirection, TransferId};
use flow_transfer::{
    Connector, RetryPolicy, TransferEngine, TransferEvent, TransferRecord, TransferSpec,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::bridge::{build_adapter, ipc_error, IpcError};

/// In-memory credential stash for live sessions. Never written to disk —
/// keychain persistence happens at connect time.
#[derive(Default)]
pub struct CredentialCache {
    entries: std::sync::Mutex<HashMap<ConnectionId, Credentials>>,
}

impl CredentialCache {
    /// Stash the credentials a session was established with.
    pub fn insert(&self, id: ConnectionId, creds: Credentials) {
        self.entries
            .lock()
            .expect("credential cache lock poisoned")
            .insert(id, creds);
    }

    /// Drop the stash for a disconnected session.
    pub fn remove(&self, id: &ConnectionId) {
        self.entries
            .lock()
            .expect("credential cache lock poisoned")
            .remove(id);
    }

    /// Clone out the credentials for a connection.
    pub fn get(&self, id: &ConnectionId) -> Option<Credentials> {
        self.entries
            .lock()
            .expect("credential cache lock poisoned")
            .get(id)
            .cloned()
    }
}

/// Engine connector dialing through the shared adapter factory.
struct AppConnector;

#[async_trait::async_trait]
impl Connector for AppConnector {
    async fn connect(&self, creds: &Credentials) -> CoreResult<Box<dyn flow_core::RemoteFs>> {
        let mut adapter = build_adapter(creds.protocol);
        adapter.connect(creds).await?;
        Ok(adapter)
    }
}

/// Build the engine, wire its event stream to the webview, and start the
/// worker pool. Call once from Tauri setup.
pub fn start_engine(app: AppHandle) -> Arc<TransferEngine> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<TransferEvent>();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(AppConnector),
        tx,
        RetryPolicy::standard(),
        None,
    ));

    // Forward engine events to the frontend.
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            let TransferEvent::Update(record) = event;
            if let Err(e) = app.emit("transfer:update", &record) {
                eprintln!("failed to emit transfer:update: {e}");
            }
        }
    });

    // One worker per concurrent transfer slot.
    for _ in 0..3 {
        let worker = engine.clone();
        tauri::async_runtime::spawn(async move { worker.run_worker().await });
    }

    engine
}

/// Request payload for enqueuing a transfer.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueRequest {
    /// Client-generated unique id for the transfer.
    pub id: String,
    pub connection_id: String,
    pub direction: TransferDirection,
    pub remote_path: String,
    pub local_path: String,
    pub file_name: String,
}

/// Enqueue an upload or download on the queue.
#[tauri::command]
pub async fn transfer_enqueue(
    engine: State<'_, Arc<TransferEngine>>,
    cache: State<'_, CredentialCache>,
    request: EnqueueRequest,
) -> Result<TransferRecord, IpcError> {
    let connection_id = ConnectionId::new(request.connection_id.clone());
    let Some(creds) = cache.get(&connection_id) else {
        return Err(IpcError {
            code: "connection-failed".into(),
            message: "That connection is no longer active. Connect again and retry.".into(),
        });
    };

    let spec = TransferSpec {
        connection_id,
        connection_name: creds.host.clone(),
        direction: request.direction,
        remote_path: FilePath::new(request.remote_path),
        local_path: FilePath::new(request.local_path),
        file_name: request.file_name,
    };

    let record = {
        // Snapshot the queued record for the immediate UI echo.
        let id = TransferId::new(request.id);
        engine.enqueue(id.clone(), spec, creds).await;
        engine
            .snapshots()
            .await
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| IpcError {
                code: "protocol".into(),
                message: "Transfer vanished immediately after enqueue.".into(),
            })?
    };
    Ok(record)
}

/// Snapshot of every tracked transfer.
#[tauri::command]
pub async fn transfer_list(
    engine: State<'_, Arc<TransferEngine>>,
) -> Result<Vec<TransferRecord>, IpcError> {
    Ok(engine.snapshots().await)
}

/// Pause a transfer (running: stop at chunk boundary; queued: leave queue).
#[tauri::command]
pub async fn transfer_pause(
    engine: State<'_, Arc<TransferEngine>>,
    id: String,
) -> Result<(), IpcError> {
    engine.pause(&TransferId::new(id)).await.map_err(ipc_error)
}

/// Resume a paused transfer from its byte offset.
#[tauri::command]
pub async fn transfer_resume(
    engine: State<'_, Arc<TransferEngine>>,
    id: String,
) -> Result<(), IpcError> {
    engine.resume(&TransferId::new(id)).await.map_err(ipc_error)
}

/// Cancel a transfer.
#[tauri::command]
pub async fn transfer_cancel(
    engine: State<'_, Arc<TransferEngine>>,
    id: String,
) -> Result<(), IpcError> {
    engine.cancel(&TransferId::new(id)).await.map_err(ipc_error)
}

/// Clear completed/failed/canceled records from the queue list.
#[tauri::command]
pub async fn transfer_clear_finished(
    engine: State<'_, Arc<TransferEngine>>,
) -> Result<(), IpcError> {
    engine.clear_finished().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::Protocol;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn app_connector_returns_a_connected_ftp_adapter() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read, mut write) = stream.into_split();
            let mut reader = BufReader::new(read);
            let mut command = String::new();

            write.write_all(b"220 Test FTP ready\r\n").await.unwrap();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "USER test-user");
            write.write_all(b"331 Password required\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "PASS test-password");
            write.write_all(b"230 Login successful\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "CWD /");
            write.write_all(b"250 Directory changed\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "PWD");
            write
                .write_all(b"257 \"/\" is current directory\r\n")
                .await
                .unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "TYPE I");
            write.write_all(b"200 Type set to I\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "SIZE README.md");
            write.write_all(b"213 12\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "MDTM README.md");
            write.write_all(b"213 20260926123456\r\n").await.unwrap();

            command.clear();
            reader.read_line(&mut command).await.unwrap();
            assert_eq!(command.trim_end(), "QUIT");
            write.write_all(b"221 Goodbye\r\n").await.unwrap();
        });

        let creds = Credentials::new("127.0.0.1", "test-user", Protocol::Ftp)
            .with_port(port)
            .with_password("test-password");
        let mut adapter = AppConnector.connect(&creds).await.unwrap();
        let file = adapter.stat(&FilePath::new("/README.md")).await.unwrap();
        assert_eq!(file.size, 12);
        adapter.disconnect().await.unwrap();
        server.await.unwrap();
    }
}
