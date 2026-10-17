//! Connection identity and status.
//!
//! A [`Connection`] is the user-facing representation of a remote endpoint —
//! the thing shown in the sidebar. Live protocol state lives in the adapter,
//! not here; this type is pure data and fully serializable.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Opaque identifier for a connection. Stable for the lifetime of the app.
/// Stored as a string to match the frontend's `string` id type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lifecycle state of a connection, surfaced to the UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    #[default]
    Disconnected,
    Error,
}

/// User-facing connection record — mirrors `Connection` in
/// `apps/desktop/src/lib/types.ts`. Serialized in camelCase so the Tauri IPC
/// boundary maps 1:1 onto the TypeScript shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: ConnectionId,
    pub name: String,
    pub protocol: crate::protocol::Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub status: ConnectionStatus,
    /// Whether the credential is backed by the OS keychain.
    #[serde(default)]
    pub keychain: bool,
    #[serde(default)]
    pub favorite: bool,
    /// Epoch milliseconds of the last successful connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Protocol;

    #[test]
    fn status_serde_lowercase() {
        let json = serde_json::to_string(&ConnectionStatus::Connecting).unwrap();
        assert_eq!(json, "\"connecting\"");
    }

    #[test]
    fn connection_round_trip() {
        let c = Connection {
            id: ConnectionId::new("c1"),
            name: "Production".into(),
            protocol: Protocol::Sftp,
            host: "prod.example.com".into(),
            port: 22,
            username: "tom".into(),
            status: ConnectionStatus::Connected,
            keychain: true,
            favorite: true,
            last_connected: Some(1_700_000_000_000),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, c.id);
        assert_eq!(back.protocol, Protocol::Sftp);
        assert!(back.keychain);
    }
}
