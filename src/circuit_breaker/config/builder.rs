//! Builder for circuit breaker configuration.

use std::time::Duration;
use crate::circuit_breaker::config::models::{CircuitBreakerConfig, SlidingWindowType};

/// Builder for `CircuitBreakerConfig`.
pub struct CircuitBreakerConfigBuilder {
    enabled: bool,
    failure_rate_threshold: f64,
    minimum_calls: u64,
    recovery_timeout: Duration,
    half_open_max_calls: usize,
    slow_call_rate_threshold: f64,
    slow_call_duration: Duration,
    sliding_window_type: SlidingWindowType,
    sliding_window_size: usize,
    call_timeout: Duration,
    max_concurrent_calls: usize,
    consecutive_failure_threshold: Option<u64>,
}

impl CircuitBreakerConfigBuilder {
    pub fn new() -> Self {
        let defaults = CircuitBreakerConfig::default();
        Self {
            enabled: defaults.enabled,
            failure_rate_threshold: defaults.failure_rate_threshold,
            minimum_calls: defaults.minimum_calls,
            recovery_timeout: defaults.recovery_timeout,
            half_open_max_calls: defaults.half_open_max_calls,
            slow_call_rate_threshold: defaults.slow_call_rate_threshold,
            slow_call_duration: defaults.slow_call_duration,
            sliding_window_type: defaults.sliding_window_type,
            sliding_window_size: defaults.sliding_window_size,
            call_timeout: defaults.call_timeout,
            max_concurrent_calls: defaults.max_concurrent_calls,
            consecutive_failure_threshold: defaults.consecutive_failure_threshold,
        }
    }

    pub fn failure_rate_threshold(mut self, threshold: f64) -> Self {
        self.failure_rate_threshold = threshold;
        self
    }

    pub fn minimum_calls(mut self, calls: u64) -> Self {
        self.minimum_calls = calls;
        self
    }

    pub fn recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    pub fn half_open_max_calls(mut self, calls: usize) -> Self {
        self.half_open_max_calls = calls;
        self
    }

    pub fn slow_call_rate_threshold(mut self, threshold: f64) -> Self {
        self.slow_call_rate_threshold = threshold;
        self
    }

    pub fn slow_call_duration(mut self, duration: Duration) -> Self {
        self.slow_call_duration = duration;
        self
    }

    pub fn sliding_window_type(mut self, window_type: SlidingWindowType) -> Self {
        self.sliding_window_type = window_type;
        self
    }

    pub fn sliding_window_size(mut self, size: usize) -> Self {
        self.sliding_window_size = size;
        self
    }

    pub fn call_timeout(mut self, timeout: Duration) -> Self {
        self.call_timeout = timeout;
        self
    }

    pub fn max_concurrent_calls(mut self, max: usize) -> Self {
        self.max_concurrent_calls = max;
        self
    }

    pub fn consecutive_failure_threshold(mut self, threshold: u64) -> Self {
        self.consecutive_failure_threshold = Some(threshold);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn build(self) -> anyhow::Result<CircuitBreakerConfig> {
        Ok(CircuitBreakerConfig {
            enabled: self.enabled,
            failure_rate_threshold: self.failure_rate_threshold,
            minimum_calls: self.minimum_calls,
            recovery_timeout: self.recovery_timeout,
            half_open_max_calls: self.half_open_max_calls,
            slow_call_rate_threshold: self.slow_call_rate_threshold,
            slow_call_duration: self.slow_call_duration,
            sliding_window_type: self.sliding_window_type,
            sliding_window_size: self.sliding_window_size,
            call_timeout: self.call_timeout,
            max_concurrent_calls: self.max_concurrent_calls,
            consecutive_failure_threshold: self.consecutive_failure_threshold,
            wait_duration_in_open_state: None,
            permitted_calls_in_half_open_state: None,
            writable_stack_trace_enabled: false,
            record_exceptions: vec![],
            ignore_exceptions: vec![],
        })
    }
}

impl Default for CircuitBreakerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
