//! Builder for `CircuitBreakerConfig`.

use std::time::Duration;
use super::{CircuitBreakerConfig, ConfigValidationError};

/// Builder for `CircuitBreakerConfig` with validation.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfigBuilder {
    pub(super) bucket_count: usize,
    pub(super) window_duration: Duration,
    pub(super) failure_rate_threshold: f64,
    pub(super) slow_call_rate_threshold: Option<f64>,
    pub(super) slow_call_duration: Option<Duration>,
    pub(super) minimum_calls: u64,
    pub(super) recovery_timeout: Duration,
    pub(super) half_open_max_calls: usize,
    pub(super) call_timeout: Option<Duration>,
    pub(super) max_concurrent_calls: usize,
    pub(super) consecutive_failure_threshold: Option<u32>,
}

impl CircuitBreakerConfigBuilder {
    pub(super) fn new() -> Self {
        let defaults = CircuitBreakerConfig::default();
        Self {
            bucket_count: defaults.bucket_count,
            window_duration: defaults.window_duration,
            failure_rate_threshold: defaults.failure_rate_threshold,
            slow_call_rate_threshold: defaults.slow_call_rate_threshold,
            slow_call_duration: defaults.slow_call_duration,
            minimum_calls: defaults.minimum_calls,
            recovery_timeout: defaults.recovery_timeout,
            half_open_max_calls: defaults.half_open_max_calls,
            call_timeout: defaults.call_timeout,
            max_concurrent_calls: defaults.max_concurrent_calls,
            consecutive_failure_threshold: defaults.consecutive_failure_threshold,
        }
    }

    /// Set the number of time buckets in the rolling window.
    pub fn bucket_count(mut self, count: usize) -> Self {
        self.bucket_count = count;
        self
    }

    /// Set the total duration the rolling window covers.
    pub fn window_duration(mut self, duration: Duration) -> Self {
        self.window_duration = duration;
        self
    }

    /// Set the failure rate threshold (0.0 to 1.0) to trip the circuit.
    pub fn failure_rate_threshold(mut self, threshold: f64) -> Self {
        self.failure_rate_threshold = threshold;
        self
    }

    /// Set the slow call rate threshold (0.0 to 1.0) to trip the circuit.
    /// Requires `slow_call_duration` to also be set.
    pub fn slow_call_rate_threshold(mut self, threshold: f64) -> Self {
        self.slow_call_rate_threshold = Some(threshold);
        self
    }

    /// Set the duration above which a call is considered slow.
    /// Requires `slow_call_rate_threshold` to also be set.
    pub fn slow_call_duration(mut self, duration: Duration) -> Self {
        self.slow_call_duration = Some(duration);
        self
    }

    /// Set the minimum number of calls before evaluating failure rate.
    pub fn minimum_calls(mut self, min: u64) -> Self {
        self.minimum_calls = min;
        self
    }

    /// Set how long the circuit stays open before transitioning to HalfOpen.
    pub fn recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    /// Set the number of probe calls allowed in HalfOpen state.
    pub fn half_open_max_calls(mut self, max: usize) -> Self {
        self.half_open_max_calls = max;
        self
    }

    /// Set the timeout applied to each call.
    pub fn call_timeout(mut self, timeout: Duration) -> Self {
        self.call_timeout = Some(timeout);
        self
    }

    /// Set the maximum concurrent calls allowed (bulkhead pattern).
    /// 0 means unlimited.
    pub fn max_concurrent_calls(mut self, max: usize) -> Self {
        self.max_concurrent_calls = max;
        self
    }

    /// Set the consecutive failure threshold to trip the circuit.
    /// This is an alternative to rate-based tripping.
    pub fn consecutive_failure_threshold(mut self, threshold: u32) -> Self {
        self.consecutive_failure_threshold = Some(threshold);
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Individual field validation
        if self.window_duration.is_zero() {
            return Err(ConfigValidationError::InvalidWindowDuration);
        }

        if self.bucket_count == 0 {
            return Err(ConfigValidationError::InvalidBucketCount);
        }

        if self.failure_rate_threshold <= 0.0 || self.failure_rate_threshold > 1.0 {
            return Err(ConfigValidationError::InvalidFailureRateThreshold);
        }

        if let Some(threshold) = self.slow_call_rate_threshold {
            if threshold <= 0.0 || threshold > 1.0 {
                return Err(ConfigValidationError::InvalidSlowCallRateThreshold);
            }
        }

        if self.slow_call_rate_threshold.is_some() {
            match self.slow_call_duration {
                Some(d) if d.is_zero() => {
                    return Err(ConfigValidationError::InvalidSlowCallDuration);
                }
                None => {
                    return Err(ConfigValidationError::InvalidSlowCallDuration);
                }
                _ => {}
            }
        }

        if self.minimum_calls == 0 {
            return Err(ConfigValidationError::InvalidMinimumCalls);
        }

        if self.recovery_timeout.is_zero() {
            return Err(ConfigValidationError::InvalidRecoveryTimeout);
        }

        if self.half_open_max_calls == 0 {
            return Err(ConfigValidationError::InvalidHalfOpenMaxCalls);
        }

        if let Some(timeout) = self.call_timeout {
            if timeout.is_zero() {
                return Err(ConfigValidationError::InvalidCallTimeout);
            }
        }

        // Cross-field validation
        if let (Some(slow_dur), Some(timeout)) = (self.slow_call_duration, self.call_timeout) {
            if slow_dur >= timeout {
                return Err(ConfigValidationError::SlowCallDurationExceedsTimeout);
            }
        }

        // Check bucket duration won't be zero
        let bucket_duration = self.window_duration / self.bucket_count as u32;
        if bucket_duration.is_zero() {
            return Err(ConfigValidationError::BucketDurationTooShort);
        }

        Ok(())
    }

    /// Build the configuration, returning an error if validation fails.
    pub fn build(self) -> Result<CircuitBreakerConfig, ConfigValidationError> {
        self.validate()?;

        Ok(CircuitBreakerConfig {
            bucket_count: self.bucket_count,
            window_duration: self.window_duration,
            failure_rate_threshold: self.failure_rate_threshold,
            slow_call_rate_threshold: self.slow_call_rate_threshold,
            slow_call_duration: self.slow_call_duration,
            minimum_calls: self.minimum_calls,
            recovery_timeout: self.recovery_timeout,
            half_open_max_calls: self.half_open_max_calls,
            call_timeout: self.call_timeout,
            max_concurrent_calls: self.max_concurrent_calls,
            consecutive_failure_threshold: self.consecutive_failure_threshold,
        })
    }
}

impl Default for CircuitBreakerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
