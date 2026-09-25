//! # flow-transfer
//!
//! FlowFTP's transfer engine: a queue drained by worker tasks, one file per
//! job, with pause/resume (byte-offset based), retry with backoff, smoothed
//! progress reporting, and optional bandwidth limiting.
//!
//! The engine dials its own connections per attempt via the [`Connector`]
//! trait, so browsing never competes with transfers for a session. Progress
//! flows out as [`TransferEvent`]s over a channel; the application layer
//! forwards them to the UI.

#![forbid(unsafe_code)]

pub mod engine;
pub mod progress;
pub mod record;

pub use engine::{Connector, ControlFlags, TransferEngine, TransferSpec};
pub use progress::{RateLimiter, RetryPolicy, SpeedEstimator};
pub use record::{TransferEvent, TransferRecord};
