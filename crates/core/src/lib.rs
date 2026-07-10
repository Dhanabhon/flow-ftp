//! # flow-core
//!
//! The domain layer of FlowFTP — pure business logic with no I/O.
//!
//! This crate defines the vocabulary every other layer speaks: protocols,
//! connections, files, transfers, sync diffs, errors, and the [`RemoteFs`]
//! trait that protocol adapters implement. It depends on nothing but `serde`
//! and `thiserror`.
//!
//! See `ARCHITECTURE.md` for how this crate fits into the Clean Architecture
//! layer model.

#![forbid(unsafe_code)]

pub mod connection;
pub mod credentials;
pub mod error;
pub mod file;
pub mod protocol;
pub mod remote;
pub mod sync;
pub mod transfer;

// Primary re-exports — the public API surface.
pub use connection::{Connection, ConnectionId, ConnectionStatus};
pub use credentials::{Credentials, SecretString};
pub use error::{CoreError, CoreResult};
pub use file::{FileKind, FilePath, RemoteFile};
pub use protocol::Protocol;
pub use remote::RemoteFs;
pub use sync::{DiffReason, SyncDiff, SyncDirection};
pub use transfer::{TransferDirection, TransferId, TransferProgress, TransferStatus};
