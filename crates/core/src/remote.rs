//! The interchange trait for remote filesystem adapters.
//!
//! `crates/protocols` provides three implementations (FTP, FTPS, SFTP). The
//! Application Layer selects one by [`crate::protocol::Protocol`] and the rest
//! of the system never knows which protocol is active — exactly the
//! interchangeability required by ARCHITECTURE.md.
//!
//! The trait is object-safe via `#[async_trait]`: the connection registry
//! stores `Box<dyn RemoteFs>`, which is what makes the application layer
//! unit-testable with a mock adapter. The dynamic-dispatch cost is negligible
//! next to a network round trip.
//!
//! Transfers stream through `open_read` / `open_write` so callers (the
//! transfer engine) own the copy loop — that is where pause/cancel control,
//! progress sampling, and bandwidth limiting live. The offset parameter makes
//! every transfer resumable: FTP maps it to `REST`/`APPE`, SFTP to a seek.
//!
//! Every method takes `&mut self`: an adapter owns a live, stateful session
//! (TCP socket, SSH channel, TLS state). Shared concurrent access is the
//! Application Layer's concern — wrap the adapter in a lock there if needed;
//! the domain keeps the contract explicit.

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::credentials::Credentials;
use crate::error::{CoreError, CoreResult};
use crate::file::{FilePath, RemoteFile};
use crate::transfer::TransferId;

/// A connected remote filesystem. Implementations own a live session for the
/// lifetime of the value.
#[async_trait]
pub trait RemoteFs: Send {
    /// Establish a session using the given credentials.
    async fn connect(&mut self, creds: &Credentials) -> CoreResult<()>;

    /// Tear down the session gracefully. Idempotent.
    async fn disconnect(&mut self) -> CoreResult<()>;

    /// List entries in a directory. The implementor may prepend a synthetic
    /// `..` entry when `path` is not the root.
    async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>>;

    /// Stat a single path.
    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile>;

    /// Create a directory (non-recursive).
    async fn mkdir(&mut self, path: &FilePath) -> CoreResult<()>;

    /// Rename/move a path.
    async fn rename(&mut self, from: &FilePath, to: &FilePath) -> CoreResult<()>;

    /// Delete a file or empty directory.
    async fn delete(&mut self, path: &FilePath) -> CoreResult<()>;

    /// Open a remote file for reading, starting at `offset` bytes in.
    /// `offset > 0` resumes an interrupted download (FTP `REST`, SFTP seek).
    async fn open_read(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncRead + Send + Unpin>>;

    /// Open a remote file for writing, starting at `offset` bytes in.
    /// `offset == 0` truncates/overwrites; `offset > 0` resumes an interrupted
    /// upload (FTP `APPE`, SFTP seek). The file is created when missing.
    async fn open_write(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>>;

    /// Download `remote` into `local`, overwriting it. Convenience over
    /// [`Self::open_read`] for callers that do not need the streaming loop.
    async fn download(
        &mut self,
        _id: TransferId,
        remote: &FilePath,
        local: &FilePath,
    ) -> CoreResult<()> {
        use tokio::io::AsyncWriteExt;
        let mut reader = self.open_read(remote, 0).await?;
        let mut file = tokio::fs::File::create(local.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        tokio::io::copy(&mut reader, &mut file)
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        file.flush().await.map_err(|e| CoreError::Io(e.to_string()))
    }

    /// Upload `local` to `remote`, overwriting it. Convenience over
    /// [`Self::open_write`] for callers that do not need the streaming loop.
    async fn upload(
        &mut self,
        _id: TransferId,
        local: &FilePath,
        remote: &FilePath,
    ) -> CoreResult<()> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.open_write(remote, 0).await?;
        let mut file = tokio::fs::File::open(local.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        tokio::io::copy(&mut file, &mut writer)
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        writer
            .flush()
            .await
            .map_err(|e| CoreError::Io(e.to_string()))
    }
}
