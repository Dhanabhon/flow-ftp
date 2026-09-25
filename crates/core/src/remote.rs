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
//!
//! Every method takes `&mut self`: an adapter owns a live, stateful session
//! (TCP socket, SSH channel, TLS state). Shared concurrent access is the
//! Application Layer's concern — wrap the adapter in a lock there if needed;
//! the domain keeps the contract explicit.

use crate::credentials::Credentials;
use crate::error::CoreResult;
use crate::file::{FilePath, RemoteFile};
use crate::transfer::TransferId;

/// A connected remote filesystem. Implementations own a live session for the
/// lifetime of the value.
pub trait RemoteFs {
    /// Establish a session using the given credentials.
    fn connect(&mut self, creds: &Credentials) -> impl Future<Output = CoreResult<()>>;

    /// Tear down the session gracefully. Idempotent.
    fn disconnect(&mut self) -> impl Future<Output = CoreResult<()>>;

    /// List entries in a directory. The implementor may prepend a synthetic
    /// `..` entry when `path` is not the root.
    fn list(&mut self, path: &FilePath) -> impl Future<Output = CoreResult<Vec<RemoteFile>>>;

    /// Stat a single path.
    fn stat(&mut self, path: &FilePath) -> impl Future<Output = CoreResult<RemoteFile>>;

    /// Create a directory (non-recursive).
    fn mkdir(&mut self, path: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Rename/move a path.
    fn rename(&mut self, from: &FilePath, to: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Delete a file or empty directory.
    fn delete(&mut self, path: &FilePath) -> impl Future<Output = CoreResult<()>>;

    /// Begin downloading `remote` into `local`. The byte stream is owned by
    /// the adapter; the domain only owns the outcome.
    fn download(
        &mut self,
        id: TransferId,
        remote: &FilePath,
        local: &FilePath,
    ) -> impl Future<Output = CoreResult<()>>;

    /// Begin uploading `local` to `remote`. See [`Self::download`].
    fn upload(
        &mut self,
        id: TransferId,
        local: &FilePath,
        remote: &FilePath,
    ) -> impl Future<Output = CoreResult<()>>;
}

// Re-export so trait users can write `use flow_core::remote::RemoteFs` without
// also needing `Future` in scope.
pub use std::future::Future;
