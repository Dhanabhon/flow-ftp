//! Wire protocol selection for a remote connection.
//!
//! Mirrors the `Protocol` type in `apps/desktop/src/lib/types.ts` so the
//! future Tauri serde bridge is a 1:1 mapping.

use serde::{Deserialize, Serialize};

/// Supported file-transfer protocols. Adapters in `crates/protocols` provide
/// one `RemoteFs` implementation per variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// Plain FTP (RFC 959).
    Ftp,
    /// FTP over TLS/SSL (RFC 4217).
    Ftps,
    /// SFTP — the SSH File Transfer Protocol.
    Sftp,
}

impl Protocol {
    /// The default TCP port for this protocol.
    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Ftp | Protocol::Ftps => {
                // FTPS defaults to 990 (implicit TLS); plain FTP to 21.
                // The connection layer resolves explicit-TLS on 21 separately.
                if matches!(self, Protocol::Ftps) {
                    990
                } else {
                    21
                }
            }
            Protocol::Sftp => 22,
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Ftp => write!(f, "ftp"),
            Protocol::Ftps => write!(f, "ftps"),
            Protocol::Sftp => write!(f, "sftp"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ports() {
        assert_eq!(Protocol::Ftp.default_port(), 21);
        assert_eq!(Protocol::Ftps.default_port(), 990);
        assert_eq!(Protocol::Sftp.default_port(), 22);
    }

    #[test]
    fn serde_round_trip_lowercase() {
        for p in [Protocol::Ftp, Protocol::Ftps, Protocol::Sftp] {
            let json = serde_json::to_string(&p).unwrap();
            assert_eq!(json, format!("\"{p}\""));
            let back: Protocol = serde_json::from_str(&json).unwrap();
            assert_eq!(back, p);
        }
    }

    #[test]
    fn display_is_lowercase() {
        assert_eq!(Protocol::Sftp.to_string(), "sftp");
    }
}
