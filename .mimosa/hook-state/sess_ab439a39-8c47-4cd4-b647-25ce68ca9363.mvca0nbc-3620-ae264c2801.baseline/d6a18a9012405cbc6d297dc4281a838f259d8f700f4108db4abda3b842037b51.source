//! # flow-protocols
//!
//! Protocol adapters for FlowFTP — concrete [`RemoteFs`](flow_core::RemoteFs)
//! implementations that the Application Layer selects by
//! [`Protocol`](flow_core::Protocol). Business logic never sees which
//! transport is underneath (see ARCHITECTURE.md).
//!
//! | Adapter | Protocol | Crate |
//! | --- | --- | --- |
//! | [`FtpFs`] | FTP | suppaftp |
//! | [`FtpsFs`] | FTPS (explicit + implicit TLS) | suppaftp + rustls |
//! | [`SftpFs`] | SFTP | russh + russh-sftp |
//!
//! Live-server integration tests are `#[ignore]`-tagged and driven by
//! environment variables so `cargo test` stays green without infrastructure.

#![forbid(unsafe_code)]

pub mod error;
pub mod ftp;
pub mod sftp;
pub mod tls;

pub use error::{map_ftp_error, map_sftp_error, map_ssh_error};
pub use ftp::{FtpsFs, FtpsMode, FtpFs};
pub use sftp::{HostKeyPolicy, SftpFs};
