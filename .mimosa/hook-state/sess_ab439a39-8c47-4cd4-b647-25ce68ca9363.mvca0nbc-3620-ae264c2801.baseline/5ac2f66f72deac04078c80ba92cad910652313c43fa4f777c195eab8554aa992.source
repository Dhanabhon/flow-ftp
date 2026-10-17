//! Integration test against the REAL macOS Keychain.
//!
//! Ignored by default: it touches the user's actual login keychain and may
//! trigger an authorization prompt. Run manually with:
//!
//! ```sh
//! cargo test -p flow-keychain --test real_keychain -- --ignored
//! ```

use flow_core::{ConnectionId, SecretString};
use flow_keychain::{KeychainStore, DEFAULT_SERVICE};

/// Unique id so repeated runs don't collide with stale entries.
fn scratch_id() -> ConnectionId {
    ConnectionId::new(format!(
        "flow-keychain-it-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before epoch")
            .as_millis()
    ))
}

#[test]
#[ignore = "touches the real macOS Keychain; may prompt for authorization"]
fn real_keychain_round_trip() {
    let store = KeychainStore::with_service(format!("{DEFAULT_SERVICE}-it"));
    let id = scratch_id();

    // Clean up even on failure paths.
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        store.save_password(&id, &SecretString::new("it-secret"))?;
        let loaded = store
            .load_password(&id)?
            .expect("saved secret must be loadable");
        assert_eq!(loaded.reveal(), "it-secret");
        Ok(())
    })();
    store.delete_password(&id).expect("cleanup delete failed");
    result.expect("round trip failed");
}
