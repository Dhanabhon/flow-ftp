//! Pure transfer math: speed estimation, rate limiting, retry backoff.
//!
//! Everything here is deterministic and unit-tested; the engine only wires
//! these into its copy loop.

use std::time::Duration;

/// Exponentially-weighted speed estimate in bytes/sec.
///
/// Sampling cadence is the caller's choice; shorter gaps between `sample`
/// calls converge faster. The estimate starts optimistic-but-sane (first
/// sample = the instantaneous rate) instead of 0.
#[derive(Debug, Clone)]
pub struct SpeedEstimator {
    alpha: f64,
    ema_bps: f64,
    last_bytes: u64,
    last_elapsed: Option<Duration>,
    seeded: bool,
}

impl SpeedEstimator {
    /// An estimator with the standard smoothing factor (0.25).
    pub fn new() -> Self {
        Self::with_alpha(0.25)
    }

    /// An estimator with an explicit smoothing factor in `0.0..=1.0`.
    /// Lower = smoother, higher = twitchier.
    pub fn with_alpha(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            ema_bps: 0.0,
            last_bytes: 0,
            last_elapsed: None,
            seeded: false,
        }
    }

    /// Feed the cumulative elapsed time and cumulative byte count so far;
    /// returns the smoothed bytes/sec estimate. The instantaneous rate is
    /// computed from the delta since the previous sample.
    pub fn sample(&mut self, elapsed: Duration, total_bytes: u64) -> u64 {
        let delta_bytes = total_bytes.saturating_sub(self.last_bytes);
        let delta_time = elapsed
            .saturating_sub(self.last_elapsed.unwrap_or_default())
            .as_secs_f64();
        self.last_bytes = total_bytes;
        self.last_elapsed = Some(elapsed);

        if !self.seeded {
            // First sample seeds the estimate directly.
            self.seeded = true;
            let instant = elapsed.as_secs_f64().max(f64::EPSILON);
            self.ema_bps = total_bytes as f64 / instant;
            return self.ema_bps as u64;
        }

        if delta_time <= f64::EPSILON {
            return self.ema_bps as u64; // no new information
        }
        let instant_bps = delta_bytes as f64 / delta_time;
        self.ema_bps += self.alpha * (instant_bps - self.ema_bps);
        self.ema_bps as u64
    }

    /// Current smoothed estimate without taking a new sample.
    pub fn estimate(&self) -> u64 {
        self.ema_bps as u64
    }
}

impl Default for SpeedEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Token-bucket bandwidth limiter.
///
/// `throttle` returns how long the caller must wait before sending the given
/// chunk to stay under the configured bytes/sec budget; `None` means send now.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    limit_bps: Option<u64>,
    budget: f64,
    last_refill: f64, // seconds
}

impl RateLimiter {
    /// A limiter with the given budget, or unlimited when `None`.
    pub fn new(limit_bps: Option<u64>) -> Self {
        Self {
            limit_bps,
            budget: limit_bps.unwrap_or(u64::MAX) as f64, // start with a full second of budget
            last_refill: 0.0,
        }
    }

    /// Whether the limiter actually constrains anything.
    pub fn is_limited(&self) -> bool {
        self.limit_bps.is_some()
    }

    /// Budget check for sending `bytes` after `elapsed` of transfer time.
    pub fn throttle(&mut self, elapsed: Duration, bytes: u64) -> Option<Duration> {
        let limit = self.limit_bps?;
        let now = elapsed.as_secs_f64();
        // Refill the bucket for the time since the last call.
        self.budget += (now - self.last_refill) * limit as f64;
        self.last_refill = now;

        let need = bytes as f64;
        if self.budget >= need {
            self.budget -= need;
            return None;
        }
        // Wait until the missing tokens accumulate.
        let deficit = need - self.budget;
        let wait = deficit / limit as f64;
        // Spending everything we have; the rest comes from the wait.
        self.budget = 0.0;
        Some(Duration::from_secs_f64(wait))
    }
}

/// Exponential retry backoff: `base * 2^(attempt-1)`, capped.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    /// A policy with `max_attempts` tries and 500 ms base delay, 30 s cap.
    pub fn standard() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }

    /// Whether another attempt is allowed after `attempt` (1-based) failures.
    pub fn should_retry(&self, failed_attempt: u32) -> bool {
        failed_attempt < self.max_attempts
    }

    /// Wait time before attempt `failed_attempt + 1`.
    pub fn delay_for(&self, failed_attempt: u32) -> Duration {
        let exp = failed_attempt.saturating_sub(1).min(16);
        let delay = self.base_delay.saturating_mul(1u32 << exp);
        delay.min(self.max_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_estimator_seeds_from_first_sample() {
        let mut est = SpeedEstimator::new();
        // 1000 bytes in 1 second → 1000 B/s
        assert_eq!(est.sample(Duration::from_secs(1), 1_000), 1_000);
    }

    #[test]
    fn speed_estimator_smooths_spikes() {
        let mut est = SpeedEstimator::new();
        est.sample(Duration::from_secs(1), 1_000); // 1000 B/s
        // Sudden burst: 100_000 bytes in the next second.
        let after_burst = est.sample(Duration::from_secs(2), 101_000); // 100_000 B/s instant
        // EMA(0.25): 1000 + 0.25 * (100000 - 1000) = 25_750
        assert_eq!(after_burst, 25_750);
    }

    #[test]
    fn speed_estimator_handles_zero_delta() {
        let mut est = SpeedEstimator::new();
        est.sample(Duration::from_secs(1), 1_000);
        // No progress: estimate decays toward 0 but never divides by zero.
        let s = est.sample(Duration::from_secs(2), 1_000);
        assert_eq!(s, 750); // 1000 + 0.25 * (0 - 1000)
    }

    #[test]
    fn rate_limiter_allows_within_budget() {
        let mut lim = RateLimiter::new(Some(1_000_000));
        // 64 KiB against a 1 MB/s budget: no wait.
        assert_eq!(lim.throttle(Duration::from_secs(1), 64 * 1024), None);
    }

    #[test]
    fn rate_limiter_throttles_over_budget() {
        let mut lim = RateLimiter::new(Some(1_000)); // 1 KB/s
        // Entire first-second budget spent on the first chunk.
        assert_eq!(lim.throttle(Duration::ZERO, 1_000), None);
        // Second chunk must wait for the bucket to refill.
        let wait = lim.throttle(Duration::ZERO, 1_000).expect("must throttle");
        assert_eq!(wait, Duration::from_secs(1));
    }

    #[test]
    fn rate_limiter_unlimited_never_throttles() {
        let mut lim = RateLimiter::new(None);
        assert_eq!(lim.throttle(Duration::ZERO, u64::MAX / 2), None);
        assert!(!lim.is_limited());
    }

    #[test]
    fn retry_policy_exponential_backoff_with_cap() {
        let policy = RetryPolicy::standard();
        assert_eq!(policy.delay_for(1), Duration::from_millis(500));
        assert_eq!(policy.delay_for(2), Duration::from_millis(1_000));
        assert_eq!(policy.delay_for(3), Duration::from_millis(2_000));
        assert!(policy.delay_for(20) <= policy.max_delay);
    }

    #[test]
    fn retry_policy_attempt_gate() {
        let policy = RetryPolicy::standard(); // 3 attempts
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }

    #[test]
    fn progress_fraction_and_eta_via_core() {
        use flow_core::TransferProgress;
        let p = TransferProgress { transferred: 500, total: 1_000 };
        assert_eq!(p.percent(), 50);
        assert_eq!(p.eta(100), Some(Duration::from_secs(5)));
    }
}
