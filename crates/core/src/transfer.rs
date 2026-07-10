//! Transfer identity, status, and progress math.
//!
//! Mirrors `Transfer` / `TransferStatus` / `TransferDirection` in
//! `apps/desktop/src/lib/types.ts`. The actual transfer engine (queue,
//! retry, resume) lives in `crates/transfer`; this module only defines the
//! domain vocabulary and the pure progress arithmetic.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Opaque transfer identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransferId(pub String);

impl TransferId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Direction of data movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Upload,
    Download,
}

/// Lifecycle state of a transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Queued,
    Active,
    Paused,
    Completed,
    Failed,
    Canceled,
}

/// Pure progress snapshot for a transfer. Derived from byte counts; the engine
/// in `crates/transfer` owns the live smoothing/ETA, but the math helpers here
/// are pure and unit-tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferProgress {
    pub transferred: u64,
    pub total: u64,
}

impl TransferProgress {
    /// Completion ratio in `0.0..=1.0`. `0.0` when the total is unknown.
    pub fn fraction(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.transferred as f64 / self.total as f64).clamp(0.0, 1.0)
        }
    }

    /// Whole-percent completion (`0..=100`).
    pub fn percent(self) -> u8 {
        (self.fraction() * 100.0).round() as u8
    }

    /// Bytes still to transfer (`total - transferred`).
    pub fn remaining(self) -> u64 {
        self.total.saturating_sub(self.transferred)
    }

    /// Estimated time remaining given a current speed in bytes/sec. `None`
    /// when the speed is zero or the total unknown.
    pub fn eta(self, bytes_per_sec: u64) -> Option<Duration> {
        if bytes_per_sec == 0 || self.total == 0 {
            return None;
        }
        Some(Duration::from_secs(self.remaining() / bytes_per_sec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_clamps() {
        let p = TransferProgress { transferred: 200, total: 100 };
        assert_eq!(p.fraction(), 1.0);
        assert_eq!(p.percent(), 100);
    }

    #[test]
    fn fraction_zero_total() {
        let p = TransferProgress { transferred: 50, total: 0 };
        assert_eq!(p.fraction(), 0.0);
        assert_eq!(p.percent(), 0);
    }

    #[test]
    fn remaining_basic() {
        let p = TransferProgress { transferred: 30, total: 100 };
        assert_eq!(p.remaining(), 70);
    }

    #[test]
    fn remaining_saturates() {
        let p = TransferProgress { transferred: 150, total: 100 };
        assert_eq!(p.remaining(), 0);
    }

    #[test]
    fn eta_zero_speed_is_none() {
        let p = TransferProgress { transferred: 0, total: 100 };
        assert!(p.eta(0).is_none());
    }

    #[test]
    fn eta_basic() {
        let p = TransferProgress { transferred: 0, total: 1_000 };
        assert_eq!(p.eta(100), Some(Duration::from_secs(10)));
    }

    #[test]
    fn direction_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&TransferDirection::Download).unwrap(),
            "\"download\""
        );
    }

    #[test]
    fn status_serde_round_trip() {
        for s in [
            TransferStatus::Queued,
            TransferStatus::Active,
            TransferStatus::Completed,
            TransferStatus::Failed,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: TransferStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }
}
