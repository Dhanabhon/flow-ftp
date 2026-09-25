//! Protocol error → domain error mapping.
//!
//! Per PRODUCT.md, raw protocol errors never reach the user: the adapters
//! translate provider-specific errors into [`CoreError`] variants that the
//! Application Layer renders as human-readable "what happened / why / how to
//! fix it" messages.

use flow_core::CoreError;
use russh::Error as SshError;
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::protocol::StatusCode;
use suppaftp::types::Response;
use suppaftp::{FtpError, Status};

/// Map an FTP error to the domain error type.
///
/// Response codes carry the semantic weight: 5xx are permanent server
/// refusals (auth, missing file, permission), 4xx transient. We match the
/// codes users actually hit and let everything else fall through with the
/// server's own text attached, so the Application Layer always has context.
pub fn map_ftp_error(e: FtpError) -> CoreError {
    match e {
        FtpError::ConnectionError(io) => CoreError::ConnectionFailed {
            host: "remote".into(),
            port: 0,
            reason: io.to_string(),
        },
        FtpError::SecureError(reason) => CoreError::ConnectionFailed {
            host: "remote".into(),
            port: 0,
            reason: format!("TLS negotiation failed: {reason}"),
        },
        FtpError::UnexpectedResponse(response) => map_response(response),
        // Syntax-level breakage means we cannot trust the conversation.
        other => CoreError::Protocol(other.to_string()),
    }
}

/// Map an unexpected server response to the closest domain error.
fn map_response(response: Response) -> CoreError {
    let text = String::from_utf8_lossy(&response.body).into_owned();
    match response.status {
        Status::NotLoggedIn | Status::InvalidCredentials => {
            CoreError::AuthFailed { username: String::new() }
        }
        Status::FileUnavailable => CoreError::NotFound(text),
        Status::RequestFileActionIgnored
        | Status::RequestedActionNotTaken
        | Status::ActionAborted
        | Status::ExceededStorage
        | Status::BadFilename => CoreError::Permission(text),
        Status::NotAvailable | Status::CannotOpenDataConnection | Status::TransferAborted => {
            CoreError::Transfer(text)
        }
        _ => CoreError::Protocol(text),
    }
}

/// Map an SSH transport error to the domain error type.
///
/// Authentication failures do not surface here — `russh` reports them as an
/// `AuthResult::Failure` value which the adapter converts itself. This maps
/// transport-level breakage.
pub fn map_ssh_error(e: SshError) -> CoreError {
    match e {
        SshError::ConnectionTimeout | SshError::KeepaliveTimeout | SshError::InactivityTimeout => {
            CoreError::Timeout(std::time::Duration::from_secs(15))
        }
        // The handshake callback rejected the host key: first contact without
        // a pin, or a fingerprint mismatch (possible machine-in-the-middle).
        SshError::UnknownKey => CoreError::ConnectionFailed {
            host: "remote".into(),
            port: 0,
            reason: "host key rejected or not pinned".into(),
        },
        SshError::IO(io) => CoreError::Io(io.to_string()),
        other => CoreError::Protocol(other.to_string()),
    }
}

/// Map an SFTP protocol error to the domain error type.
pub fn map_sftp_error(e: SftpError) -> CoreError {
    match e {
        SftpError::Status(status) => map_sftp_status(status),
        SftpError::Timeout => CoreError::Timeout(std::time::Duration::from_secs(15)),
        SftpError::IO(msg) => CoreError::Io(msg),
        other => CoreError::Protocol(other.to_string()),
    }
}

/// Map an SFTP status code to the closest domain error.
fn map_sftp_status(status: russh_sftp::protocol::Status) -> CoreError {
    let text = status.error_message;
    match status.status_code {
        StatusCode::NoSuchFile => CoreError::NotFound(text),
        StatusCode::PermissionDenied => CoreError::Permission(text),
        StatusCode::NoConnection | StatusCode::ConnectionLost => CoreError::Transfer(text),
        _ => CoreError::Protocol(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(code: Status) -> FtpError {
        FtpError::UnexpectedResponse(suppaftp::types::Response::new(code, Vec::new()))
    }

    #[test]
    fn auth_failure_maps_to_auth_failed() {
        assert!(matches!(
            map_ftp_error(response(Status::NotLoggedIn)),
            CoreError::AuthFailed { .. }
        ));
        assert!(matches!(
            map_ftp_error(response(Status::InvalidCredentials)),
            CoreError::AuthFailed { .. }
        ));
    }

    #[test]
    fn file_unavailable_maps_to_not_found() {
        let e = map_ftp_error(response(Status::FileUnavailable));
        assert!(matches!(e, CoreError::NotFound(_)));
    }

    #[test]
    fn connection_error_maps_to_connection_failed() {
        let e = map_ftp_error(FtpError::ConnectionError(std::io::Error::other("refused")));
        assert!(matches!(e, CoreError::ConnectionFailed { .. }));
    }

    #[test]
    fn unknown_code_maps_to_protocol_with_context() {
        let e = map_ftp_error(response(Status::BadCommand));
        assert!(matches!(e, CoreError::Protocol(_)));
    }

    #[test]
    fn secure_error_explains_tls() {
        let e = map_ftp_error(FtpError::SecureError("bad cert".into()));
        match e {
            CoreError::ConnectionFailed { reason, .. } => {
                assert!(reason.contains("TLS"));
                assert!(reason.contains("bad cert"));
            }
            _ => panic!("expected ConnectionFailed"),
        }
    }
}
