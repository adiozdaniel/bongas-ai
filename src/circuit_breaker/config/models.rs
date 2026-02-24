//! Circuit breaker configuration models.

use std::time::Duration;
use super::builder::CircuitBreakerConfigBuilder;

/// Configuration for a circuit breaker instance.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub(crate) bucket_count: usize,
    pub(crate) window_duration: Duration,
    pub(crate) failure_rate_threshold: f64,
    pub(crate) slow_call_rate_threshold: Option<f64>,
    pub(crate) slow_call_duration: Option<Duration>,
    pub(crate) minimum_calls: u64,
    pub(crate) recovery_timeout: Duration,
    pub(crate) half_open_max_calls: usize,
    pub(crate) call_timeout: Option<Duration>,
    pub(crate) max_concurrent_calls: usize,
    pub(crate) consecutive_failure_threshold: Option<u32>,
}

impl CircuitBreakerConfig {
    /// Start building a configuration.
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }

    // ─── Accessors ─────────────────────────────────────────────────────────

    #[inline]
    pub fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    #[inline]
    pub fn window_duration(&self) -> Duration {
        self.window_duration
    }

    #[inline]
    pub fn bucket_duration(&self) -> Duration {
        self.window_duration / self.bucket_count as u32
    }

    #[inline]
    pub fn failure_rate_threshold(&self) -> f64 {
        self.failure_rate_threshold
    }

    #[inline]
    pub fn slow_call_rate_threshold(&self) -> Option<f64> {
        self.slow_call_rate_threshold
    }

    #[inline]
    pub fn slow_call_duration(&self) -> Option<Duration> {
        self.slow_call_duration
    }

    #[inline]
    pub fn minimum_calls(&self) -> u64 {
        self.minimum_calls
    }

    #[inline]
    pub fn recovery_timeout(&self) -> Duration {
        self.recovery_timeout
    }

    #[inline]
    pub fn half_open_max_calls(&self) -> usize {
        self.half_open_max_calls
    }

    #[inline]
    pub fn call_timeout(&self) -> Option<Duration> {
        self.call_timeout
    }

    #[inline]
    pub fn max_concurrent_calls(&self) -> usize {
        self.max_concurrent_calls
    }

    #[inline]
    pub fn consecutive_failure_threshold(&self) -> Option<u32> {
        self.consecutive_failure_threshold
    }

    /// Returns true if slow call detection is enabled.
    #[inline]
    pub fn slow_call_detection_enabled(&self) -> bool {
        self.slow_call_rate_threshold.is_some() && self.slow_call_duration.is_some()
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            bucket_count: 10,
            window_duration: Duration::from_secs(10),
            failure_rate_threshold: 0.5,
            slow_call_rate_threshold: None,
            slow_call_duration: None,
            minimum_calls: 5,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
            call_timeout: None,
            max_concurrent_calls: 0,
            consecutive_failure_threshold: None,
        }
    }
}
