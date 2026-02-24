//! Rolling window data models.

/// Snapshot of the rolling window's aggregate counters.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct WindowSnapshot {
    pub successes: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub slow_calls: u64,
    pub rejections: u64,
    /// Total calls that completed (success + failure + timeout).
    pub total_calls: u64,
    /// Failure rate: (failures + timeouts) / total_calls.
    pub failure_rate: f64,
    /// Slow call rate: slow_calls / total_calls.
    pub slow_call_rate: f64,
    /// Average latency in microseconds (0 if no calls).
    pub avg_latency_us: u64,
}

impl WindowSnapshot {
    /// Returns true if the window has enough data for evaluation.
    #[inline]
    pub fn has_minimum_calls(&self, minimum: u64) -> bool {
        self.total_calls >= minimum
    }

    /// Returns true if failure rate exceeds the threshold.
    #[inline]
    pub fn exceeds_failure_threshold(&self, threshold: f64) -> bool {
        self.failure_rate >= threshold
    }

    /// Returns true if slow call rate exceeds the threshold.
    #[inline]
    pub fn exceeds_slow_call_threshold(&self, threshold: f64) -> bool {
        self.slow_call_rate >= threshold
    }
}
