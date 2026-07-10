//! The interchange trait for remote filesystem adapters.
//!
//! `crates/protocols` provides three implementations (FTP, FTPS, SFTP). The
//! Application Layer selects one by [`crate::protocol::Protocol`] and the rest
//! of the system never knows which protocol is active — exactly the
//! interchangeability required by ARCHITECTURE.md.
//!
//! Uses native `async fn` in traits (stable since Rust 1.75; the toolchain
//! here is 1.96), so no `async-trait` macro dependency is needed. The trade-off
//! is that `RemoteFs` cannot be used as a `dyn` trait object directly; the
//! application layer holds concrete adapters behind an enum dispatch instead.

use crate::credentials::Credentials;
use crate::error::CoreResult;
use crate::file::{FilePath, RemoteFile};
use crate::transfer::TransferId;

/// A connected remote filesystem. Implementations own a live session (TCP
/// socket, SSH channel, etc.) for the lifetime of the value.
pub trait RemoteFs {
    /// Establish a session using the given credentials.
    fn connect(&mut self, creds: &Credentials) -> impl Future<Output = CoreResult<()>>;

    /// Tear down the session gracefully. Idempotent.
    fn disconnect(&mut self) -> impl Future<Output = CoreResult<()>>;

    /// List entries in a directory. The implementor may prepend a synthetic
    /// `..` entry when `path` is not the root.
    fn list(&self, path: &FilePath) -> impl Future<Output = CoreResult<Vec<RemoteFile>>>;

    /// Stat a single path.
    fn stat(&self, path: &FilePath) -> impl Future<Output = CoreResult<RemoteFile>>;

    /// Create a directory (non-recursive).
    fn mkdir(&self, path: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Rename/move a path.
    fn rename(&self, from: &FilePath, to: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Delete a file or empty directory.
    fn delete(&self, path: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Begin downloading `remote` into `local`. Returns an id the engine uses
    /// to track progress. The actual byte stream is owned by the adapter /
    /// transfer crate; the domain only owns the id.
    fn download(
        &self,
        id: TransferId,
        remote: &FilePath,
        local: &FilePath,
    ) -> impl Future<Output = CoreResult<()>>;

    /// Begin uploading `local` to `remote`. See [`Self::download`].
    fn upload(
        &self,
        id: TransferId,
        local: &FilePath,
        remote: &FilePath,
    ) -> impl Future<Output = CoreResult<()>>;
}

// Re-export so trait users can write `use flow_core::remote::RemoteFs` without
// also needing `Future` in scope.
pub use std::future::Future;
