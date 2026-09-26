//! FTP and FTPS adapters over [suppaftp](https://crates.io/crates/suppaftp).
//!
//! [`FtpFs`] speaks plain FTP; [`FtpsFs`] speaks FTP over TLS in both flavors
//! — explicit (`AUTH TLS` on the control port, the modern default) and
//! implicit (TLS from the first byte, the legacy port-990 mode). Both
//! implement the domain's [`RemoteFs`] trait so they are interchangeable with
//! the SFTP adapter behind it.
//!
//! The command layer is shared and generic over the concrete stream type:
//! suppaftp bounds every command on its own `TokioTlsStream` trait, which
//! both the plain and TLS streams implement.

use std::time::Duration;

use async_trait::async_trait;
use flow_core::{
    Credentials, CoreError, CoreResult, FileKind, FilePath, RemoteFile, RemoteFs,
};
use suppaftp::tokio::{
    AsyncFtpStream, AsyncRustlsFtpStream, ImplAsyncFtpStream, TokioTlsStream,
};
use tokio::io::{AsyncRead, AsyncWrite};
use suppaftp::list::ListParser;
use suppaftp::types::FileType as FtpFileType;
use suppaftp::FtpError;

use crate::error::map_ftp_error;
use crate::tls::tls_connector;

/// How long a full connect (TCP + TLS + login) may take. PRODUCT.md targets a
/// sub-2s connection; servers on far latencies get this ceiling instead.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Plain-FTP control stream (no TLS).
type PlainStream = AsyncFtpStream;
/// FTPS control stream (rustls TLS).
type TlsStream = AsyncRustlsFtpStream;

// ─────────────────────────────────────────────────────────────────────────────
// Shared command layer — generic over the concrete stream type.
// ─────────────────────────────────────────────────────────────────────────────

/// Log in on an established control connection.
async fn login<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    creds: &Credentials,
) -> CoreResult<()> {
    let password = creds
        .password
        .as_ref()
        .map(|s| s.reveal().to_string())
        .unwrap_or_default();
    stream
        .login(&creds.username, &password)
        .await
        .map_err(|e| match e {
            // Login refusals carry the username for the UI message.
            FtpError::UnexpectedResponse(_) => CoreError::AuthFailed {
                username: creds.username.clone(),
            },
            other => map_ftp_error(other),
        })?;
    log::debug!("login OK as {}", creds.username);
    Ok(())
}

/// CWD into a directory before running a command against its entries.
///
/// Every mainstream client (Cyberduck, FileZilla) works this way: CWD into
/// the directory, then address entries by bare name. Some shared-hosting FTP
/// servers mishandle absolute paths in LIST/RETR/SIZE — one was observed
/// listing the home directory for `LIST /domains` and answering 450 for the
/// doubled path — while the same operations behave correctly after a CWD.
async fn enter_dir<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    dir: &FilePath,
) -> CoreResult<()> {
    let dir = dir.as_str();
    stream.cwd(dir).await.map_err(|e| {
        let mapped = map_ftp_error(e);
        match mapped {
            CoreError::Protocol(text) => CoreError::Protocol(format!("CWD {dir}: {text}")),
            other => other,
        }
    })?;
    // PWD is the server's own idea of where we are. Logging it after every
    // CWD makes a server that acknowledges the command but lands elsewhere
    // visible in the transcript.
    match stream.pwd().await {
        Ok(pwd) => log::debug!("CWD {dir} -> PWD {pwd}"),
        Err(e) => log::debug!("CWD {dir} -> PWD failed: {e}"),
    }
    Ok(())
}

/// Select binary transfer mode for file metadata and contents.
async fn use_binary_type<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
) -> CoreResult<()> {
    stream
        .transfer_type(FtpFileType::Image)
        .await
        .map_err(map_ftp_error)
}

/// List a directory. Prefers MLSD (RFC 3659, machine-readable) and falls back
/// to the classic LIST parser for servers without it. The directory is
/// entered with CWD first (see [`enter_dir`]) and listed relatively; a LIST
/// failure is the real error and is annotated with the command and path for
/// diagnosis (e.g. symlinks a server refuses to follow).
async fn list_entries<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    path: &FilePath,
) -> CoreResult<Vec<RemoteFile>> {
    enter_dir(stream, path).await?;
    let mut method = "MLSD";
    let entries: Vec<RemoteFile> = match stream.mlsd(None).await {
        Ok(lines) => lines
            .iter()
            .filter_map(|line| ListParser::parse_mlsd(line).ok())
            .map(to_remote_file)
            .collect(),
        Err(_) => {
            method = "LIST";
            stream
                .list(None)
                .await
                .map_err(|e| {
                    let mapped = map_ftp_error(e);
                    match mapped {
                        CoreError::Protocol(text) => {
                            CoreError::Protocol(format!("LIST {path}: {text}"))
                        }
                        other => other,
                    }
                })?
                .iter()
                .filter_map(|line| ListParser::parse_posix(line).ok())
                .map(to_remote_file)
                .collect()
        }
    };
    let preview: Vec<String> = entries
        .iter()
        .take(3)
        .map(|f| f.name.chars().take(24).collect())
        .collect();
    log::debug!(
        "LIST {}: {} entries via {}: {}",
        path.as_str(),
        entries.len(),
        method,
        preview.join(", ")
    );
    Ok(entries)
}

/// Stat one path via SIZE + MDTM (universally supported; MLST is not).
/// Runs relative to the parent directory after a CWD (see [`enter_dir`]).
async fn stat_path<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    path: &FilePath,
) -> CoreResult<RemoteFile> {
    let name = path.name().to_string();
    enter_dir(stream, &path.parent()).await?;
    use_binary_type(stream).await?;
    let size = stream
        .size(name.as_str())
        .await
        .map_err(map_ftp_error)? as u64;
    let modified = stream
        .mdtm(name.as_str())
        .await
        .ok()
        .map(|dt| dt.and_utc().timestamp_millis())
        .unwrap_or(0);
    Ok(RemoteFile {
        name,
        kind: FileKind::File,
        size,
        modified,
        permissions: None,
        owner: None,
        group: None,
    })
}

/// Delete a path: files go via DELE, directories via RMD. Trying file first
/// and falling back on a server refusal mirrors typical FTP clients.
async fn delete_any<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    path: &FilePath,
) -> CoreResult<()> {
    let name = path.name().to_string();
    enter_dir(stream, &path.parent()).await?;
    match stream.rm(name.as_str()).await {
        Ok(()) => Ok(()),
        Err(FtpError::UnexpectedResponse(_)) => {
            stream.rmdir(name.as_str()).await.map_err(map_ftp_error)
        }
        Err(e) => Err(map_ftp_error(e)),
    }
}

/// Open a remote file for reading, optionally resuming from `offset`
/// (FTP `REST` before `RETR`). The file is addressed relatively after a CWD
/// into its parent (see [`enter_dir`]).
async fn open_read_stream<T: TokioTlsStream + Send + 'static>(
    stream: &mut ImplAsyncFtpStream<T>,
    remote: &FilePath,
    offset: u64,
) -> CoreResult<Box<dyn AsyncRead + Send + Unpin>> {
    let name = remote.name().to_string();
    enter_dir(stream, &remote.parent()).await?;
    use_binary_type(stream).await?;
    if offset > 0 {
        stream
            .resume_transfer(offset as usize)
            .await
            .map_err(map_ftp_error)?;
    }
    let data = stream
        .retr_as_stream(name.as_str())
        .await
        .map_err(map_ftp_error)?;
    Ok(Box::new(data))
}

/// Open a remote file for writing. `offset == 0` stores fresh (STOR);
/// `offset > 0` appends (APPE) so interrupted uploads resume. The file is
/// addressed relatively after a CWD into its parent (see [`enter_dir`]).
async fn open_write_stream<T: TokioTlsStream + Send + 'static>(
    stream: &mut ImplAsyncFtpStream<T>,
    remote: &FilePath,
    offset: u64,
) -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>> {
    let name = remote.name().to_string();
    enter_dir(stream, &remote.parent()).await?;
    use_binary_type(stream).await?;
    let data = if offset > 0 {
        stream.append_with_stream(name.as_str()).await
    } else {
        stream.put_with_stream(name.as_str()).await
    }
    .map_err(map_ftp_error)?;
    Ok(Box::new(data))
}

/// Create a directory, relative to its parent after a CWD (see
/// [`enter_dir`]).
async fn mkdir_in<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    path: &FilePath,
) -> CoreResult<()> {
    let name = path.name().to_string();
    enter_dir(stream, &path.parent()).await?;
    stream.mkdir(name.as_str()).await.map_err(map_ftp_error)
}

/// Rename within one directory via bare names after a CWD (see
/// [`enter_dir`]). Cross-directory moves keep absolute paths, which RNFR /
/// RNTO accept on servers that resolve them correctly.
async fn rename_to<T: TokioTlsStream + Send>(
    stream: &mut ImplAsyncFtpStream<T>,
    from: &FilePath,
    to: &FilePath,
) -> CoreResult<()> {
    if from.parent().as_str() == to.parent().as_str() {
        let dir = from.parent();
        enter_dir(stream, &dir).await?;
        stream
            .rename(from.name(), to.name())
            .await
            .map_err(map_ftp_error)
    } else {
        stream
            .rename(from.as_str(), to.as_str())
            .await
            .map_err(map_ftp_error)
    }
}

/// Convert a suppaftp list entry into the domain type.
fn to_remote_file(entry: suppaftp::list::File) -> RemoteFile {
    let kind = if entry.is_directory() {
        FileKind::Directory
    } else if entry.is_symlink() {
        FileKind::Symlink
    } else {
        FileKind::File
    };
    RemoteFile {
        name: entry.name().to_string(),
        kind,
        size: entry.size() as u64,
        modified: entry
            .modified()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
        permissions: None,
        owner: None,
        group: None,
    }
}

/// Map a TCP/TLS dial error with host context attached.
fn dial_error(host: &str, port: u16, e: FtpError) -> CoreError {
    match map_ftp_error(e) {
        CoreError::ConnectionFailed { reason, .. } => CoreError::ConnectionFailed {
            host: host.to_string(),
            port,
            reason,
        },
        other => other,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FtpFs — plain FTP (RFC 959).
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for plain FTP. Holds the live control connection once
/// [`RemoteFs::connect`] succeeds.
pub struct FtpFs {
    stream: Option<PlainStream>,
}

impl FtpFs {
    /// A disconnected adapter.
    pub fn new() -> Self {
        Self { stream: None }
    }

    /// The live stream, or a domain error if not connected.
    fn require(&mut self) -> CoreResult<&mut PlainStream> {
        self.stream
            .as_mut()
            .ok_or_else(|| CoreError::Protocol("not connected".into()))
    }
}

impl Default for FtpFs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteFs for FtpFs {
    async fn connect(&mut self, creds: &Credentials) -> CoreResult<()> {
        let addr = format!("{}:{}", creds.host, creds.port);
        let mut stream: PlainStream = tokio::time::timeout(
            CONNECT_TIMEOUT,
            AsyncFtpStream::connect(addr.as_str()),
        )
        .await
        .map_err(|_| CoreError::Timeout(CONNECT_TIMEOUT))?
        .map_err(|e| dial_error(&creds.host, creds.port, e))?;
        login(&mut stream, creds).await?;
        self.stream = Some(stream);
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        if let Some(mut stream) = self.stream.take() {
            // Best effort: a dead socket must not fail the disconnect.
            let _ = stream.quit().await;
        }
        Ok(())
    }

    async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>> {
        list_entries(self.require()?, path).await
    }

    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile> {
        stat_path(self.require()?, path).await
    }

    async fn mkdir(&mut self, path: &FilePath) -> CoreResult<()> {
        mkdir_in(self.require()?, path).await
    }

    async fn rename(&mut self, from: &FilePath, to: &FilePath) -> CoreResult<()> {
        rename_to(self.require()?, from, to).await
    }

    async fn delete(&mut self, path: &FilePath) -> CoreResult<()> {
        delete_any(self.require()?, path).await
    }

    async fn open_read(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncRead + Send + Unpin>> {
        open_read_stream(self.require()?, remote, offset).await
    }

    async fn open_write(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>> {
        open_write_stream(self.require()?, remote, offset).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FtpsFs — FTP over TLS, explicit and implicit.
// ─────────────────────────────────────────────────────────────────────────────

/// TLS handshake flavor for FTPS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtpsMode {
    /// `AUTH TLS` on the control port (usually 21) — modern servers.
    Explicit,
    /// TLS from the first byte (usually port 990) — legacy servers.
    Implicit,
}

/// Adapter for FTP over TLS.
pub struct FtpsFs {
    mode: FtpsMode,
    stream: Option<TlsStream>,
}

impl FtpsFs {
    /// An adapter using explicit TLS (`AUTH TLS`).
    pub fn explicit() -> Self {
        Self {
            mode: FtpsMode::Explicit,
            stream: None,
        }
    }

    /// An adapter using implicit TLS from the first byte.
    pub fn implicit() -> Self {
        Self {
            mode: FtpsMode::Implicit,
            stream: None,
        }
    }

    /// The live stream, or a domain error if not connected.
    fn require(&mut self) -> CoreResult<&mut TlsStream> {
        self.stream
            .as_mut()
            .ok_or_else(|| CoreError::Protocol("not connected".into()))
    }
}

#[async_trait]
impl RemoteFs for FtpsFs {
    async fn connect(&mut self, creds: &Credentials) -> CoreResult<()> {
        let addr = format!("{}:{}", creds.host, creds.port);
        let connector = tls_connector()?;
        let domain = creds.host.clone();

        let mut stream: TlsStream = match self.mode {
            FtpsMode::Implicit => tokio::time::timeout(
                CONNECT_TIMEOUT,
                TlsStream::connect_secure_implicit(addr.as_str(), connector, &domain),
            )
            .await
            .map_err(|_| CoreError::Timeout(CONNECT_TIMEOUT))?
            .map_err(|e| dial_error(&creds.host, creds.port, e))?,
            FtpsMode::Explicit => {
                // suppaftp types the pre-upgrade stream as the TLS flavor
                // already: connect over plain TCP with T = rustls stream,
                // then AUTH TLS upgrades in place.
                let plain: TlsStream = tokio::time::timeout(
                    CONNECT_TIMEOUT,
                    TlsStream::connect(addr.as_str()),
                )
                .await
                .map_err(|_| CoreError::Timeout(CONNECT_TIMEOUT))?
                .map_err(|e| dial_error(&creds.host, creds.port, e))?;
                plain
                    .into_secure(connector, &domain)
                    .await
                    .map_err(map_ftp_error)?
            }
        };

        login(&mut stream, creds).await?;
        self.stream = Some(stream);
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.quit().await;
        }
        Ok(())
    }

    async fn list(&mut self, path: &FilePath) -> CoreResult<Vec<RemoteFile>> {
        list_entries(self.require()?, path).await
    }

    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile> {
        stat_path(self.require()?, path).await
    }

    async fn mkdir(&mut self, path: &FilePath) -> CoreResult<()> {
        mkdir_in(self.require()?, path).await
    }

    async fn rename(&mut self, from: &FilePath, to: &FilePath) -> CoreResult<()> {
        rename_to(self.require()?, from, to).await
    }

    async fn delete(&mut self, path: &FilePath) -> CoreResult<()> {
        delete_any(self.require()?, path).await
    }

    async fn open_read(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncRead + Send + Unpin>> {
        open_read_stream(self.require()?, remote, offset).await
    }

    async fn open_write(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>> {
        open_write_stream(self.require()?, remote, offset).await
    }
}
