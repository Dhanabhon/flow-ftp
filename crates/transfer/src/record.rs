//! Transfer records and events crossing to the UI.
//!
//! [`TransferRecord`] mirrors the TypeScript `Transfer` type
//! (`apps/desktop/src/lib/types.ts`) 1:1 in camelCase; the frontend listens
//! for [`TransferEvent`] snapshots and renders them directly.

use flow_core::{TransferDirection, TransferId, TransferStatus};
use serde::Serialize;

/// Full snapshot of one transfer's user-visible state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    pub id: TransferId,
    pub file_name: String,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    /// Total bytes to move (`0` while unknown).
    pub size: u64,
    /// Bytes moved so far.
    pub transferred: u64,
    /// Smoothed bytes/sec estimate (`0` when idle).
    pub speed: u64,
    /// Seconds remaining at the current speed (`None` when unknown).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_secs: Option<u64>,
    pub connection_id: String,
    pub connection_name: String,
    pub remote_path: String,
    pub local_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Whether an interrupted transfer can continue from its byte offset.
    pub resumable: bool,
}

impl TransferRecord {
    /// Seconds remaining given the current speed, when both are known.
    pub fn compute_eta(&self) -> Option<u64> {
        if self.speed == 0 || self.size == 0 {
            return None;
        }
        Some(self.size.saturating_sub(self.transferred) / self.speed)
    }
}

/// Something the engine wants the UI to know about.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TransferEvent {
    /// A record was created or its state changed. Carries the full snapshot —
    /// the frontend simply replaces by id.
    Update(TransferRecord),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_serializes_camel_case() {
        let record = TransferRecord {
            id: TransferId::new("t1"),
            file_name: "app.js".into(),
            direction: TransferDirection::Upload,
            status: TransferStatus::Active,
            size: 100,
            transferred: 40,
            speed: 10,
            eta_secs: Some(6),
            connection_id: "c1".into(),
            connection_name: "Prod".into(),
            remote_path: "/var/www/app.js".into(),
            local_path: "~/app.js".into(),
            started_at: Some(1),
            finished_at: None,
            error: None,
            resumable: true,
        };
        let json = serde_json::to_string(&record).unwrap();
        for key in ["fileName", "connectionId", "remotePath", "startedAt", "etaSecs"] {
            assert!(json.contains(&format!("\"{key}\"")), "missing {key} in {json}");
        }
        assert!(!json.contains("finishedAt"), "None fields are skipped");
    }

    #[test]
    fn eta_computation() {
        let mut record = TransferRecord {
            id: TransferId::new("t1"),
            file_name: "f".into(),
            direction: TransferDirection::Download,
            status: TransferStatus::Active,
            size: 1_000,
            transferred: 250,
            speed: 50,
            eta_secs: None,
            connection_id: "c".into(),
            connection_name: "c".into(),
            remote_path: "/f".into(),
            local_path: "f".into(),
            started_at: None,
            finished_at: None,
            error: None,
            resumable: false,
        };
        assert_eq!(record.compute_eta(), Some(15));
        record.speed = 0;
        assert_eq!(record.compute_eta(), None);
    }
}
