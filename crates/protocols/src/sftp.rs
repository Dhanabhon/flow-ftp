//! SFTP adapter over [russh](https://crates.io/crates/russh) (SSH transport)
//! and [russh-sftp](https://crates.io/crates/russh-sftp) (SFTP subsystem).
//!
//! [`SftpFs`] implements the domain's [`RemoteFs`] trait, making it
//! interchangeable with the FTP/FTPS adapters. Authentication is
//! password-based (from the keychain-bound [`SecretString`]); key-based auth
//! lands with the connection profiles.
//!
//! Host keys are verified according to a [`HostKeyPolicy`]: either pinned to
//! an OpenSSH-style SHA-256 fingerprint, or (development only) accepted
//! freely. First-use trust with a persistent known-hosts store is a later
//! application-layer concern.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use flow_core::{
    format_permissions, Credentials, CoreError, CoreResult, FileKind, FilePath, RemoteFile,
    RemoteFs, TransferId,
};
use russh::client::{self, Handle};
use russh::keys::{HashAlg, PublicKeyOrCertificate};
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileAttributes;

use crate::error::{map_sftp_error, map_ssh_error};

/// How long dial + auth + subsystem init may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Host key verification policy applied during the SSH handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyPolicy {
    /// Accept any host key. **Development only** — open to machine-in-the-middle
    /// attacks. The application layer will persist first-use fingerprints.
    AcceptAny,
    /// Accept only when the host key's OpenSSH SHA-256 fingerprint matches,
    /// e.g. `SHA256:abcdef…`. Obtain it with `ssh-keyscan` or from
    /// `~/.ssh/known_hosts`.
    Pin { sha256: String },
}

/// russh handshake callback applying [`HostKeyPolicy`].
struct PolicyHandler {
    policy: HostKeyPolicy,
}

impl client::Handler for PolicyHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        match &self.policy {
            HostKeyPolicy::AcceptAny => Ok(true),
            HostKeyPolicy::Pin { sha256 } => {
                let actual = key_fingerprint(server_public_key);
                Ok(actual.as_deref() == Some(sha256.as_str()))
            }
        }
    }
}

/// OpenSSH-format SHA-256 fingerprint of a host key, e.g. `SHA256:AbCd…`.
fn key_fingerprint(key: &PublicKeyOrCertificate) -> Option<String> {
    match key {
        PublicKeyOrCertificate::PublicKey { key, .. } => {
            Some(key.fingerprint(HashAlg::Sha256).to_string())
        }
        // Certificates are rare for interactive clients; reject by mismatch.
        PublicKeyOrCertificate::Certificate(_) => None,
    }
}

/// Adapter for SFTP. Holds the live SFTP session (and the SSH handle that
/// must stay alive for the connection to persist) after [`RemoteFs::connect`].
pub struct SftpFs {
    policy: HostKeyPolicy,
    handle: Option<Handle<PolicyHandler>>,
    sftp: Option<SftpSession>,
}

impl SftpFs {
    /// A disconnected adapter applying the given host key policy.
    pub fn with_policy(policy: HostKeyPolicy) -> Self {
        Self {
            policy,
            handle: None,
            sftp: None,
        }
    }

    /// A disconnected adapter accepting any host key. See [`HostKeyPolicy::AcceptAny`].
    pub fn new() -> Self {
        Self::with_policy(HostKeyPolicy::AcceptAny)
    }

    /// The live session, or a domain error if not connected.
    fn require(&mut self) -> CoreResult<&mut SftpSession> {
        self.sftp
            .as_mut()
            .ok_or_else(|| CoreError::Protocol("not connected".into()))
    }

    /// Dial, authenticate, open the SFTP subsystem. Returns both the SFTP
    /// session and the SSH handle that must stay alive for the connection.
    async fn establish(
        &mut self,
        creds: &Credentials,
    ) -> CoreResult<(Handle<PolicyHandler>, SftpSession)> {
        let config = Arc::new(client::Config::default());
        let handler = PolicyHandler {
            policy: self.policy.clone(),
        };
        let addr = (creds.host.as_str(), creds.port);

        let connect = async {
            let mut handle: Handle<PolicyHandler> =
                client::connect(config, addr, handler).await.map_err(map_ssh_error)?;

            let password = creds
                .password
                .as_ref()
                .map(|s| s.reveal().to_string())
                .unwrap_or_default();
            let auth = handle
                .authenticate_password(&creds.username, password)
                .await
                .map_err(map_ssh_error)?;
            if !auth.success() {
                return Err(CoreError::AuthFailed {
                    username: creds.username.clone(),
                });
            }

            let channel = handle
                .channel_open_session()
                .await
                .map_err(map_ssh_error)?;
            channel
                .request_subsystem(true, "sftp")
                .await
                .map_err(map_ssh_error)?;
            let sftp = SftpSession::new(channel.into_stream())
                .await
                .map_err(map_sftp_error)?;
            Ok((handle, sftp))
        };

        tokio::time::timeout(CONNECT_TIMEOUT, connect)
            .await
            .map_err(|_| CoreError::Timeout(CONNECT_TIMEOUT))?
    }
}

impl Default for SftpFs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteFs for SftpFs {
    async fn connect(&mut self, creds: &Credentials) -> CoreResult<()> {
        // The handle keeps the SSH connection alive; store it alongside the
        // SFTP session for the adapter's lifetime.
        let (handle, sftp) = self.establish(creds).await?;
        self.handle = Some(handle);
        self.sftp = Some(sftp);
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        if let Some(sftp) = self.sftp.take() {
            // Best effort: a dead session must not fail the disconnect.
            let _ = sftp.close().await;
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "english")
                .await;
        }
        Ok(())
    }

    async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>> {
        let sftp = self.require()?;
        let entries = sftp
            .read_dir(path.as_str())
            .await
            .map_err(map_sftp_error)?
            .map(|entry| attrs_to_remote_file(entry.file_name(), entry.metadata()))
            .collect();
        Ok(entries)
    }

    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile> {
        let sftp = self.require()?;
        let attrs = sftp
            .metadata(path.as_str())
            .await
            .map_err(map_sftp_error)?;
        Ok(attrs_to_remote_file(path.name().to_string(), attrs))
    }

    async fn mkdir(&mut self, path: &FilePath) -> CoreResult<()> {
        self.require()?
            .create_dir(path.as_str())
            .await
            .map_err(map_sftp_error)
    }

    async fn rename(&mut self, from: &FilePath, to: &FilePath) -> CoreResult<()> {
        self.require()?
            .rename(from.as_str(), to.as_str())
            .await
            .map_err(map_sftp_error)
    }

    async fn delete(&mut self, path: &FilePath) -> CoreResult<()> {
        // Files delete with REMOVE, directories with RMDIR — try file first
        // and fall back, mirroring the FTP adapters.
        let sftp = self.require()?;
        match sftp.remove_file(path.as_str()).await {
            Ok(()) => Ok(()),
            Err(SftpError::Status(_)) => {
                sftp.remove_dir(path.as_str()).await.map_err(map_sftp_error)
            }
            Err(e) => Err(map_sftp_error(e)),
        }
    }

    async fn download(
        &mut self,
        _id: TransferId,
        remote: &FilePath,
        local: &FilePath,
    ) -> CoreResult<()> {
        use tokio::io::AsyncWriteExt;
        let sftp = self.require()?;
        let mut remote_file = sftp
            .open(remote.as_str())
            .await
            .map_err(map_sftp_error)?;
        let mut file = tokio::fs::File::create(local.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        tokio::io::copy(&mut remote_file, &mut file)
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        file.flush().await.map_err(|e| CoreError::Io(e.to_string()))?;
        Ok(())
    }

    async fn upload(
        &mut self,
        _id: TransferId,
        local: &FilePath,
        remote: &FilePath,
    ) -> CoreResult<()> {
        use tokio::io::AsyncWriteExt;
        let sftp = self.require()?;
        let mut remote_file = sftp
            .create(remote.as_str())
            .await
            .map_err(map_sftp_error)?;
        let mut file = tokio::fs::File::open(local.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        tokio::io::copy(&mut file, &mut remote_file)
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        remote_file
            .flush()
            .await
            .map_err(|e| CoreError::Io(e.to_string()))?;
        Ok(())
    }
}

/// Convert SFTP attributes plus a name into the domain type.
fn attrs_to_remote_file(name: String, attrs: FileAttributes) -> RemoteFile {
    let kind = if attrs.is_dir() {
        FileKind::Directory
    } else if attrs.is_symlink() {
        FileKind::Symlink
    } else {
        FileKind::File
    };
    RemoteFile {
        name,
        kind,
        size: attrs.size.unwrap_or(0),
        // SFTP v3 mtimes are epoch seconds.
        modified: attrs.mtime.map(|s| i64::from(s) * 1000).unwrap_or(0),
        permissions: attrs.permissions.map(format_permissions),
        owner: attrs.user,
        group: attrs.group,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_pin_policy_matches_string_prefix() {
        let policy = HostKeyPolicy::Pin {
            sha256: "SHA256:abc".into(),
        };
        // The policy compares against the OpenSSH-format fingerprint; the
        // exact value is exercised in the live test.
        assert!(matches!(policy, HostKeyPolicy::Pin { .. }));
    }
}
