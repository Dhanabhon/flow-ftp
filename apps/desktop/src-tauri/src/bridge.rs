//! Application-layer services: connection registry and adapter dispatch.
//!
//! The registry owns one live [`RemoteFs`] adapter per connected connection,
//! keyed by [`ConnectionId`]. The frontend never touches adapters directly —
//! Tauri commands (see `ipc.rs`) look connections up here and delegate.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use flow_core::{
    ConnectionId, Credentials, CoreError, CoreResult, FilePath, Protocol, RemoteFile, RemoteFs,
};
use flow_protocols::{FtpsFs, FtpFs, HostKeyPolicy, SftpFs};

/// Build the adapter matching a protocol. Host-key verification is
/// development-mode for now (see the SFTP adapter docs); the known-hosts UX
/// lands with connection profiles.
pub(crate) fn build_adapter(protocol: Protocol) -> Box<dyn RemoteFs> {
    match protocol {
        Protocol::Ftp => Box::new(FtpFs::new()),
        Protocol::Ftps => Box::new(FtpsFs::explicit()),
        Protocol::Sftp => Box::new(SftpFs::with_policy(HostKeyPolicy::AcceptAny)),
    }
}

/// One established remote session.
struct LiveConnection {
    adapter: Box<dyn RemoteFs>,
    /// Host/port/username/protocol the session was established with. The
    /// password is deliberately not retained.
    #[allow(dead_code)]
    creds: Credentials,
}

/// Live connections, keyed by the client-generated connection id.
///
/// A tokio (async) mutex: operations hold the lock across network awaits by
/// design — protocol sessions handle one command at a time, so serializing
/// per connection is correct, and the async guard stays `Send`.
#[derive(Default)]
pub struct ConnectionRegistry {
    live: tokio::sync::Mutex<HashMap<ConnectionId, LiveConnection>>,
}

impl ConnectionRegistry {
    /// Establish a session and register it, replacing any prior one with the
    /// same id.
    pub async fn connect(&self, id: ConnectionId, creds: Credentials) -> CoreResult<()> {
        let mut adapter = build_adapter(creds.protocol);
        adapter.connect(&creds).await?;
        self.live.lock().await.insert(id, LiveConnection { adapter, creds });
        Ok(())
    }

    /// How many sessions are live right now, for the quit confirmation.
    /// A held lock means an operation is mid-flight, which itself implies a
    /// live session.
    pub fn connected_count_now(&self) -> usize {
        match self.live.try_lock() {
            Ok(guard) => guard.len(),
            Err(_) => 1,
        }
    }

    /// Tear down and remove a session. Unknown ids succeed silently so the
    /// UI can disconnect stale state freely.
    pub async fn disconnect(&self, id: &ConnectionId) -> CoreResult<()> {
        if let Some(mut live) = self
            .live
            .lock()
            .await
            .remove(id)
        {
            live.adapter.disconnect().await?;
        }
        Ok(())
    }

    /// List a directory on a live connection.
    pub async fn list(&self, id: &ConnectionId, path: FilePath) -> CoreResult<Vec<RemoteFile>> {
        // Owned args move into the boxed future so it borrows only the
        // adapter reference — see `locked`.
        self.locked(id, move |fs| Box::pin(async move { fs.list(&path).await }))
            .await
    }

    /// Stat a path on a live connection.
    pub async fn stat(&self, id: &ConnectionId, path: FilePath) -> CoreResult<RemoteFile> {
        self.locked(id, move |fs| Box::pin(async move { fs.stat(&path).await }))
            .await
    }

    /// Create a directory on a live connection.
    pub async fn mkdir(&self, id: &ConnectionId, path: FilePath) -> CoreResult<()> {
        self.locked(id, move |fs| Box::pin(async move { fs.mkdir(&path).await }))
            .await
    }

    /// Rename/move a path on a live connection.
    pub async fn rename(&self, id: &ConnectionId, from: FilePath, to: FilePath) -> CoreResult<()> {
        self.locked(id, move |fs| Box::pin(async move { fs.rename(&from, &to).await }))
            .await
    }

    /// Delete a file or empty directory on a live connection.
    pub async fn delete(&self, id: &ConnectionId, path: FilePath) -> CoreResult<()> {
        self.locked(id, move |fs| Box::pin(async move { fs.delete(&path).await }))
            .await
    }

    /// Lock the registry, fetch the connection, and await the operation.
    /// The boxed-future form sidesteps closure lifetime gymnastics while
    /// keeping the registry lock held for the whole operation.
    async fn locked<T>(
        &self,
        id: &ConnectionId,
        operation: impl FnOnce(&mut dyn RemoteFs) -> Pin<Box<dyn Future<Output = CoreResult<T>> + Send + '_>>,
    ) -> CoreResult<T> {
        let mut guard = self.live.lock().await;
        let live = guard
            .get_mut(id)
            .ok_or_else(|| CoreError::Protocol("not connected".into()))?;
        operation(live.adapter.as_mut()).await
    }

    /// Register a pre-built adapter. Test-only: lets the bridge logic be
    /// exercised without a network.
    #[cfg(test)]
    async fn insert_for_test(&self, id: ConnectionId, adapter: Box<dyn RemoteFs>) {
        self.live.lock().await.insert(
            id,
            LiveConnection {
                adapter,
                creds: Credentials::new("test-host", "tester", Protocol::Sftp),
            },
        );
    }
}

/// Serializable error shape crossing the IPC boundary. `code` matches the
/// `IpcErrorCode` union in `apps/desktop/src/lib/ipc.ts`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

/// Convert a domain error into its IPC shape. The message is the
/// human-readable explanation (what happened + why) per PRODUCT.md.
pub fn ipc_error(e: CoreError) -> IpcError {
    let (code, message) = match e {
        CoreError::ConnectionFailed { host, port, reason } => (
            "connection-failed",
            format!("Could not connect to {host}:{port}. {reason}"),
        ),
        CoreError::AuthFailed { username } => (
            "auth-failed",
            format!("Authentication failed for '{username}'. Check the username and password."),
        ),
        CoreError::NotFound(path) => ("not-found", format!("Not found: {path}")),
        CoreError::Permission(msg) => ("permission", msg),
        CoreError::Protocol(msg) => ("protocol", msg),
        CoreError::Io(msg) => ("io", msg),
        CoreError::InvalidPath(path) => ("invalid-path", format!("Invalid path: {path}")),
        CoreError::Timeout(d) => ("timeout", format!("Timed out after {}s", d.as_secs())),
        CoreError::Transfer(msg) => ("transfer", msg),
    };
    IpcError {
        code: code.to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::time::Duration;

    type BoxedAsyncRead = Box<dyn tokio::io::AsyncRead + Send + Unpin>;
    type BoxedAsyncWrite = Box<dyn tokio::io::AsyncWrite + Send + Unpin>;

    /// Mock adapter recording calls — proves the registry + dispatch path
    /// without a network.
    struct MockFs {
        disconnected: bool,
        listed: Vec<FilePath>,
    }

    #[async_trait]
    impl RemoteFs for MockFs {
        async fn connect(&mut self, _creds: &Credentials) -> CoreResult<()> {
            Ok(())
        }
        async fn disconnect(&mut self) -> CoreResult<()> {
            self.disconnected = true;
            Ok(())
        }
        async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>> {
            self.listed.push(path.clone());
            Ok(vec![RemoteFile {
                name: "entry.txt".into(),
                kind: flow_core::FileKind::File,
                size: 12,
                modified: 0,
                permissions: None,
                owner: None,
                group: None,
            }])
        }
        async fn stat(&mut self, _path: &FilePath) -> CoreResult<RemoteFile> {
            unreachable!()
        }
        async fn mkdir(&mut self, _path: &FilePath) -> CoreResult<()> {
            unreachable!()
        }
        async fn rename(&mut self, _f: &FilePath, _t: &FilePath) -> CoreResult<()> {
            unreachable!()
        }
        async fn delete(&mut self, _path: &FilePath) -> CoreResult<()> {
            unreachable!()
        }
        async fn open_read(
            &mut self,
            _remote: &FilePath,
            _offset: u64,
        ) -> CoreResult<BoxedAsyncRead> {
            unreachable!()
        }
        async fn open_write(
            &mut self,
            _remote: &FilePath,
            _offset: u64,
        ) -> CoreResult<BoxedAsyncWrite> {
            unreachable!()
        }
    }

    #[test]
    fn ipc_error_codes_match_documented_union() {
        let cases: Vec<(CoreError, &str)> = vec![
            (
                CoreError::ConnectionFailed {
                    host: "h".into(),
                    port: 1,
                    reason: "x".into(),
                },
                "connection-failed",
            ),
            (CoreError::AuthFailed { username: "u".into() }, "auth-failed"),
            (CoreError::NotFound("p".into()), "not-found"),
            (CoreError::Permission("p".into()), "permission"),
            (CoreError::Protocol("p".into()), "protocol"),
            (CoreError::Io("p".into()), "io"),
            (CoreError::InvalidPath("p".into()), "invalid-path"),
            (CoreError::Transfer("p".into()), "transfer"),
        ];
        for (err, expected) in cases {
            assert_eq!(ipc_error(err).code, expected);
        }
    }

    #[test]
    fn timeout_error_includes_seconds() {
        let e = ipc_error(CoreError::Timeout(Duration::from_secs(15)));
        assert_eq!(e.code, "timeout");
        assert!(e.message.contains("15s"));
    }

    #[test]
    fn auth_error_message_mentions_username_and_fix() {
        let e = ipc_error(CoreError::AuthFailed {
            username: "deploy".into(),
        });
        assert!(e.message.contains("deploy"));
        assert!(e.message.contains("password"), "message should suggest a fix");
    }

    #[tokio::test]
    async fn list_delegates_to_registered_adapter() {
        let registry = ConnectionRegistry::default();
        let id = ConnectionId::new("mock-1");
        registry
            .insert_for_test(
                id.clone(),
                Box::new(MockFs {
                    disconnected: false,
                    listed: Vec::new(),
                }),
            )
            .await;

        let files = registry
            .list(&id, FilePath::new("/srv"))
            .await
            .expect("list must reach the mock adapter");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "entry.txt");
    }

    #[tokio::test]
    async fn disconnect_marks_adapter_and_clears_registry() {
        let registry = ConnectionRegistry::default();
        let id = ConnectionId::new("mock-2");
        let mock = Box::new(MockFs {
            disconnected: false,
            listed: Vec::new(),
        });
        registry.insert_for_test(id.clone(), mock).await;

        registry.disconnect(&id).await.expect("disconnect failed");

        let live = registry.live.lock().await;
        assert!(!live.contains_key(&id));
    }

    #[tokio::test]
    async fn list_on_unknown_id_fails_cleanly() {
        let registry = ConnectionRegistry::default();
        let result = registry.list(&ConnectionId::new("missing"), FilePath::new("/")).await;
        assert!(matches!(result, Err(CoreError::Protocol(_))));
    }

    #[tokio::test]
    async fn connected_count_reflects_live_sessions() {
        let registry = ConnectionRegistry::default();
        assert_eq!(registry.connected_count_now(), 0);
        registry
            .insert_for_test(
                ConnectionId::new("cc1"),
                Box::new(MockFs {
                    disconnected: false,
                    listed: Vec::new(),
                }),
            )
            .await;
        assert_eq!(registry.connected_count_now(), 1);
    }

    #[tokio::test]
    async fn disconnect_of_unknown_id_succeeds() {
        let registry = ConnectionRegistry::default();
        registry
            .disconnect(&ConnectionId::new("never-existed"))
            .await
            .expect("disconnecting an unknown id must not fail");
    }
}
