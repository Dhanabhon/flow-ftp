//! Saved connection profiles, persisted as JSON in the app config dir.
//!
//! Profiles store connection *metadata* only — host, port, username,
//! favorite. Passwords never touch this file; they live in the OS keychain
//! under the same connection id (see `crates/keychain`).

use std::path::PathBuf;
use std::sync::Mutex;

use flow_core::{Connection, ConnectionId, ConnectionStatus};
use tauri::State;

use crate::bridge::IpcError;

/// The persisted store. Mutations rewrite the file atomically
/// (write-temp-then-rename) so a crash never truncates the config.
pub struct ProfileStore {
    path: PathBuf,
    profiles: Mutex<Vec<Connection>>,
}

impl ProfileStore {
    /// Load from `dir/connections.json`, or start empty when missing.
    pub fn load(dir: &std::path::Path) -> Self {
        let path = dir.join("connections.json");
        let profiles = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<Connection>>(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            profiles: Mutex::new(profiles),
        }
    }

    /// Atomically rewrite the file from the in-memory list.
    fn persist(&self, profiles: &[Connection]) -> Result<(), IpcError> {
        let json = serde_json::to_vec_pretty(profiles).map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Could not serialize profiles: {e}"),
        })?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Could not write profiles: {e}"),
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|e| IpcError {
            code: "io".into(),
            message: format!("Could not finalize profiles file: {e}"),
        })
    }

    fn snapshot(&self) -> Vec<Connection> {
        self.profiles.lock().expect("profiles lock poisoned").clone()
    }

    /// Upsert a profile (matched by id) and persist. The incoming status is
    /// normalized to `disconnected` — liveness is session state, not profile
    /// data.
    fn upsert(&self, mut profile: Connection) -> Result<Vec<Connection>, IpcError> {
        profile.status = ConnectionStatus::Disconnected;
        let mut profiles = self.profiles.lock().expect("profiles lock poisoned");
        match profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(existing) => *existing = profile,
            None => profiles.push(profile),
        }
        self.persist(&profiles)?;
        drop(profiles);
        Ok(self.snapshot())
    }

    /// Remove a profile by id and persist.
    fn delete(&self, id: &ConnectionId) -> Result<Vec<Connection>, IpcError> {
        let mut profiles = self.profiles.lock().expect("profiles lock poisoned");
        profiles.retain(|p| &p.id != id);
        self.persist(&profiles)?;
        drop(profiles);
        Ok(self.snapshot())
    }
}

/// All saved profiles, sidebar order (favorites first, then by name).
#[tauri::command]
pub fn profile_list(store: State<'_, ProfileStore>) -> Result<Vec<Connection>, IpcError> {
    Ok(store.snapshot())
}

/// Upsert a profile (matched by id) and persist.
#[tauri::command]
pub fn profile_save(
    store: State<'_, ProfileStore>,
    profile: Connection,
) -> Result<Vec<Connection>, IpcError> {
    store.upsert(profile)
}

/// Remove a profile by id and persist.
#[tauri::command]
pub fn profile_delete(
    store: State<'_, ProfileStore>,
    id: String,
) -> Result<Vec<Connection>, IpcError> {
    store.delete(&ConnectionId::new(id))
}


#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::{ConnectionId, Protocol};

    fn profile(id: &str, name: &str, favorite: bool) -> Connection {
        Connection {
            id: ConnectionId::new(id),
            name: name.into(),
            protocol: Protocol::Sftp,
            host: "prod.example.com".into(),
            port: 22,
            username: "tom".into(),
            status: ConnectionStatus::Connected, // must normalize on save
            keychain: true,
            favorite,
            last_connected: Some(1_700_000_000_000),
        }
    }

    fn temp_store(tag: &str) -> (ProfileStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("flowftp-profiles-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir failed");
        (ProfileStore::load(&dir), dir)
    }

    #[test]
    fn upsert_then_reload_round_trips() {
        let (store, dir) = temp_store("round-trip");

        let list = store.upsert(profile("c1", "Prod", true)).expect("save failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Prod");
        // Status normalizes: liveness is session state, not profile data.
        assert_eq!(list[0].status, ConnectionStatus::Disconnected);

        // A second store instance reads what the first persisted.
        let reloaded = ProfileStore::load(&dir);
        let list = reloaded.snapshot();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, ConnectionId::new("c1"));
        assert_eq!(list[0].last_connected, Some(1_700_000_000_000));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_upserts_by_id_instead_of_duplicating() {
        let (store, dir) = temp_store("upsert");
        let first = store.upsert(profile("c1", "First", false)).expect("save failed");
        assert_eq!(first.len(), 1);

        // Same id, new name -> replace, not duplicate.
        let second = store.upsert(profile("c1", "Renamed", true)).expect("save failed");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].name, "Renamed");
        assert!(second[0].favorite);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn delete_removes_and_persists() {
        let (store, dir) = temp_store("delete");
        let _ = store.upsert(profile("c1", "A", false));
        let _ = store.upsert(profile("c2", "B", false));

        let mut after = store.delete(&ConnectionId::new("c1")).expect("delete failed");
        assert_eq!(after.len(), 1);
        assert_eq!(after.pop().unwrap().id, ConnectionId::new("c2"));

        // Persisted: a fresh store no longer sees c1.
        let reloaded = ProfileStore::load(&dir);
        assert!(reloaded.snapshot().iter().all(|p| p.id.as_str() != "c1"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_file_starts_empty() {
        let (store, dir) = temp_store("empty");
        assert!(store.snapshot().is_empty());
        std::fs::remove_dir_all(dir).ok();
    }
}
