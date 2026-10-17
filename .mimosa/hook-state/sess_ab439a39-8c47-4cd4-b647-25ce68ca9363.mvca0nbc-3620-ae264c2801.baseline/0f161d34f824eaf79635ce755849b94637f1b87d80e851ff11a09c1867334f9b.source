//! Credential handling for remote connections.
//!
//! `SecretString` wraps a password so it cannot be accidentally logged or
//! serialized. It deliberately omits `Serialize` and provides a redacting
//! `Debug` impl — the only way to read the inner value is via
//! [`SecretString::reveal`], which lives at the keychain boundary.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::protocol::Protocol;

/// A password or other secret. Redacts in debug output and is never
/// serialized — credentials travel through `crates/keychain`, not JSON.
///
/// `Deserialize` is implemented so config files / keychain payloads can round
/// a secret *into* the domain, but `Serialize` is deliberately omitted so a
/// `SecretString` can never leave the process via serde by accident.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct SecretString(#[serde(deserialize_with = "deserialize_secret")] String);

/// Deserialize any value as a secret string. Accepts a plain string or a
/// map containing `password` so multiple keychain/config formats work.
fn deserialize_secret<'de, D: serde::Deserializer<'de>>(de: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Plain(String),
        Map { password: String },
    }
    let repr = Repr::deserialize(de)?;
    Ok(match repr {
        Repr::Plain(s) => s,
        Repr::Map { password } => password,
    })
}

impl SecretString {
    /// Wrap a secret value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// View the inner secret. Use only at trust boundaries (keychain I/O,
    /// protocol auth).
    pub fn reveal(&self) -> &str {
        &self.0
    }

    /// Whether the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(**REDACTED**)")
    }
}

/// Connection parameters sufficient to dial a remote host.
///
/// `password` is optional to support key-based SSH auth and anonymous FTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default, skip_serializing)]
    pub password: Option<SecretString>,
    pub protocol: Protocol,
}

impl Credentials {
    /// Build credentials, defaulting the port to the protocol's standard port.
    pub fn new(host: impl Into<String>, username: impl Into<String>, protocol: Protocol) -> Self {
        let host = host.into();
        let port = protocol.default_port();
        Self {
            host,
            port,
            username: username.into(),
            password: None,
            protocol,
        }
    }

    /// Attach a password.
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(SecretString::new(password));
        self
    }

    /// Attach an explicit port.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_redacts_in_debug() {
        let secret = SecretString::new("hunter2");
        let debug = format!("{secret:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("hunter2"));
    }

    #[test]
    fn secret_string_reveal_works() {
        let secret = SecretString::new("hunter2");
        assert_eq!(secret.reveal(), "hunter2");
        assert!(!secret.is_empty());
    }

    #[test]
    fn credentials_default_port() {
        let c = Credentials::new("example.com", "tom", Protocol::Sftp);
        assert_eq!(c.port, 22);
    }

    #[test]
    fn credentials_builder() {
        let c = Credentials::new("example.com", "tom", Protocol::Ftp)
            .with_port(2121)
            .with_password("pw");
        assert_eq!(c.port, 2121);
        assert_eq!(c.password.as_ref().unwrap().reveal(), "pw");
    }

    #[test]
    fn credentials_omits_password_from_json() {
        let c = Credentials::new("example.com", "tom", Protocol::Sftp).with_password("pw");
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("pw"));
        assert!(!json.contains("password"));
        // Round-trips back with password=None (skipped).
        let back: Credentials = serde_json::from_str(&json).unwrap();
        assert!(back.password.is_none());
    }
}
