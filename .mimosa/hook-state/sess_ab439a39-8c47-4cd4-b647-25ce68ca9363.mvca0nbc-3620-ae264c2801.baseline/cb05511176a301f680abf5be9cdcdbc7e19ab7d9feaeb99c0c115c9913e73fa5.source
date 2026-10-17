//! Folder-sync domain types.
//!
//! Only the *vocabulary* lives here. The actual comparison engine (diff by
//! size/mtime/checksum) is implemented in `crates/sync` during Phase 4.

use serde::{Deserialize, Serialize};

use crate::transfer::TransferDirection;

/// Reconciliation direction for a sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncDirection {
    /// Reconcile both sides.
    #[serde(rename = "both")]
    Bidirectional,
    /// Push local changes to remote.
    #[serde(rename = "local-to-remote")]
    LocalToRemote,
    /// Pull remote changes to local.
    #[serde(rename = "remote-to-local")]
    RemoteToLocal,
}

/// Why a path appears in the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffReason {
    /// Source side has a newer mtime.
    Newer,
    /// Missing on the target side entirely.
    Missing,
    /// Same path, different size.
    Larger,
    /// Both sides changed — needs a decision.
    Conflict,
}

/// A single proposed change from a folder comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncDiff {
    pub path: String,
    pub direction: TransferDirection,
    pub size: u64,
    pub reason: DiffReason,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_serde_matches_frontend() {
        // Frontend expects 'both' | 'local-to-remote' | 'remote-to-local'.
        assert_eq!(
            serde_json::to_string(&SyncDirection::Bidirectional).unwrap(),
            "\"both\""
        );
        assert_eq!(
            serde_json::to_string(&SyncDirection::LocalToRemote).unwrap(),
            "\"local-to-remote\""
        );
        assert_eq!(
            serde_json::to_string(&SyncDirection::RemoteToLocal).unwrap(),
            "\"remote-to-local\""
        );
    }

    #[test]
    fn diff_round_trip() {
        let d = SyncDiff {
            path: "/public/index.html".into(),
            direction: TransferDirection::Upload,
            size: 14_200,
            reason: DiffReason::Newer,
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: SyncDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
