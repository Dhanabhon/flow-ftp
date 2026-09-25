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
//! Every method takes `&mut self`: an adapter owns a live, stateful session
//! (TCP socket, SSH channel, TLS state). Shared concurrent access is the
//! Application Layer's concern — wrap the adapter in a lock there if needed;
//! the domain keeps the contract explicit.

use async_trait::async_trait;

use crate::credentials::Credentials;
use crate::error::CoreResult;
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

    /// Download `remote` into `local`. The byte stream is owned by the
    /// adapter; the domain only owns the outcome.
    async fn download(
        &mut self,
        id: TransferId,
        remote: &FilePath,
        local: &FilePath,
    ) -> CoreResult<()>;

    /// Upload `local` to `remote`. See [`Self::download`].
    async fn upload(
        &mut self,
        id: TransferId,
        local: &FilePath,
        remote: &FilePath,
    ) -> CoreResult<()>;
}
