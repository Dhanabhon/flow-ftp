//! Domain errors for FlowFTP.
//!
//! Infrastructure layers (protocol adapters, keychain) convert their native
//! errors into these variants. The Application Layer then maps `CoreError`
//! into human-readable messages for the UI — raw protocol errors never reach
//! the user (see PRODUCT.md: "Every error explains what happened, why, and
//! how to fix it").

use thiserror::Error;

/// The single error type for all domain operations.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Could not connect to {host}:{port}: {reason}")]
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },

    #[error("Authentication failed for user '{username}'")]
    AuthFailed { username: String },

    #[error("Path not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Input/output error: {0}")]
    Io(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Transfer failed: {0}")]
    Transfer(String),
}

/// Convenience alias used throughout the domain layer.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_human_readable() {
        let e = CoreError::ConnectionFailed {
            host: "example.com".into(),
            port: 22,
            reason: "connection refused".into(),
        };
        assert!(e.to_string().contains("example.com:22"));
        assert!(e.to_string().contains("connection refused"));
    }

    #[test]
    fn auth_error_mentions_username() {
        let e = CoreError::AuthFailed {
            username: "deploy".into(),
        };
        assert!(e.to_string().contains("deploy"));
    }
}
