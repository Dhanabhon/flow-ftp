//! # flow-keychain
//!
//! Secure credential storage for FlowFTP, backed by the operating system
//! keychain (macOS Keychain via Security.framework).
//!
//! Only *secrets* live here — connection metadata (host, port, username,
//! protocol) belongs to the application's config store. The store is keyed by
//! [`ConnectionId`] so a connection's password survives renames of its
//! user-facing fields but is dropped with the connection itself.
//!
//! Tests use keyring's in-memory mock store; a single `#[ignore]`-tagged test
//! exercises the real macOS keychain (run manually with
//! `cargo test -p flow-keychain -- --ignored`).

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Mutex, Once};

use flow_core::{ConnectionId, CoreError, CoreResult, SecretString};

/// Default keychain service name. Matches the Tauri bundle identifier so the
/// entry is recognizable in Keychain Access.
pub const DEFAULT_SERVICE: &str = "com.flowftp.app";

/// Stores and retrieves connection secrets in the OS keychain.
///
/// All methods are synchronous; wrap in `spawn_blocking` at the application
/// layer if calling from async context.
///
/// Keyring entry handles are cached per connection: some backends (and
/// keyring's own mock) keep the credential state inside the entry handle, so
/// re-creating the handle per call would lose written data. The cache holds
/// only the handle — the underlying secret is still read from and written to
/// the real backend on every operation.
pub struct KeychainStore {
    service: String,
    entries: Mutex<HashMap<String, keyring::Entry>>,
}

impl KeychainStore {
    /// A store using [`DEFAULT_SERVICE`].
    pub fn new() -> Self {
        Self::with_service(DEFAULT_SERVICE)
    }

    /// A store using a custom service name (useful for tests and profiles).
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Persist the secret for a connection, replacing any previous value.
    pub fn save_password(
        &self,
        connection_id: &ConnectionId,
        secret: &SecretString,
    ) -> CoreResult<()> {
        self.with_entry(connection_id, |entry| {
            entry.set_password(secret.reveal()).map_err(to_core_error)
        })
    }

    /// Load the secret for a connection. `Ok(None)` when nothing is stored.
    pub fn load_password(
        &self,
        connection_id: &ConnectionId,
    ) -> CoreResult<Option<SecretString>> {
        self.with_entry(connection_id, |entry| match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::new(password))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(to_core_error(e)),
        })
    }

    /// Remove the secret for a connection. Idempotent: deleting a missing
    /// entry succeeds.
    pub fn delete_password(&self, connection_id: &ConnectionId) -> CoreResult<()> {
        self.with_entry(connection_id, |entry| {
            match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(to_core_error(e)),
            }
        })
    }

    /// Run an operation with the cached (or freshly created) keyring entry
    /// for a connection.
    fn with_entry<T>(
        &self,
        connection_id: &ConnectionId,
        operation: impl FnOnce(&keyring::Entry) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut entries = self
            .entries
            .lock()
            .expect("keychain entry cache lock poisoned");
        let key = connection_id.as_str();
        if !entries.contains_key(key) {
            let entry =
                keyring::Entry::new(&self.service, key).map_err(to_core_error)?;
            entries.insert(key.to_string(), entry);
        }
        // Invariant: the branch above guarantees the key is present.
        let entry = entries
            .get(key)
            .expect("keyring entry was just inserted into the cache");
        operation(entry)
    }
}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a keyring error into the domain error type.
///
/// A free function rather than `impl From<keyring::Error> for CoreError`:
/// both types are foreign to this crate, so the orphan rule forbids that
/// impl here. (The keychain crate is the natural owner of this mapping —
/// it is the layer that knows keyring.)
fn to_core_error(e: keyring::Error) -> CoreError {
    match e {
        // The keychain (or the app's access to it) is locked or the user
        // denied access — a permissions problem, not an I/O one.
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            CoreError::Permission(format!(
                "Keychain access failed. Unlock the keychain or grant FlowFTP access. ({e})"
            ))
        }
        keyring::Error::NoEntry => {
            CoreError::NotFound("credential not found in keychain".into())
        }
        // Anything else is an unexpected storage-level failure. The enum is
        // #[non_exhaustive], so a catch-all keeps future keyring versions
        // compiling.
        _ => CoreError::Io(format!("Keychain storage error: {e}")),
    }
}

/// Install keyring's in-memory mock store as the default credential backend.
/// Used by tests so they never touch the real keychain. Idempotent.
pub fn use_mock_backend() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique id per test — the mock store is process-global.
    fn test_id(tag: &str) -> ConnectionId {
        ConnectionId::new(format!("test-{tag}-{}", std::process::id()))
    }

    #[test]
    fn save_then_load_round_trips() {
        use_mock_backend();
        let store = KeychainStore::with_service("flow-test");
        let id = test_id("round-trip");

        store
            .save_password(&id, &SecretString::new("hunter2"))
            .expect("save failed");
        let loaded = store.load_password(&id).expect("load failed");
        assert_eq!(
            loaded.map(|s| s.reveal().to_string()),
            Some("hunter2".into())
        );
    }

    #[test]
    fn save_overwrites_previous_value() {
        use_mock_backend();
        let store = KeychainStore::with_service("flow-test");
        let id = test_id("overwrite");

        store.save_password(&id, &SecretString::new("first")).unwrap();
        store.save_password(&id, &SecretString::new("second")).unwrap();
        let loaded = store.load_password(&id).unwrap();
        assert_eq!(loaded.map(|s| s.reveal().to_string()), Some("second".into()));
    }

    #[test]
    fn load_missing_returns_none() {
        use_mock_backend();
        let store = KeychainStore::with_service("flow-test");
        let id = test_id("missing");

        let loaded = store.load_password(&id).expect("load should not error");
        assert!(loaded.is_none());
    }

    #[test]
    fn delete_is_idempotent() {
        use_mock_backend();
        let store = KeychainStore::with_service("flow-test");
        let id = test_id("delete");

        store.save_password(&id, &SecretString::new("pw")).unwrap();
        store.delete_password(&id).expect("first delete failed");
        store.delete_password(&id).expect("second delete failed");
        assert!(store.load_password(&id).unwrap().is_none());
    }

    #[test]
    fn distinct_connections_are_isolated() {
        use_mock_backend();
        let store = KeychainStore::with_service("flow-test");
        let a = test_id("iso-a");
        let b = test_id("iso-b");

        store.save_password(&a, &SecretString::new("pw-a")).unwrap();
        store.save_password(&b, &SecretString::new("pw-b")).unwrap();
        assert_eq!(store.load_password(&a).unwrap().unwrap().reveal(), "pw-a");
        assert_eq!(store.load_password(&b).unwrap().unwrap().reveal(), "pw-b");
    }

    #[test]
    fn keyring_error_maps_to_core_error() {
        let mapped = to_core_error(keyring::Error::NoStorageAccess(Box::new(
            std::io::Error::other("locked"),
        )));
        assert!(matches!(mapped, CoreError::Permission(_)));
        assert!(mapped.to_string().contains("Keychain"));

        let mapped = to_core_error(keyring::Error::NoEntry);
        assert!(matches!(mapped, CoreError::NotFound(_)));

        // Unknown / future variants fall through to Io, not panic.
        let mapped = to_core_error(keyring::Error::TooLong("service".into(), 255));
        assert!(matches!(mapped, CoreError::Io(_)));
    }
}
