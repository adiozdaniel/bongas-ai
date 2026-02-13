//! Circuit breaker configuration with Builder pattern.
//!
//! Provides ergonomic, validated configuration for circuit breakers.
//! Every parameter has a sensible default based on Netflix Hystrix
//! production recommendations.

use std::time::Duration;

/// Configuration for a circuit breaker instance.
///
/// All fields have defaults suitable for most production workloads.
/// Use `CircuitBreakerConfig::builder()` for ergonomic construction.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of time buckets in the rolling window.
    /// More buckets = finer granularity. Default: 10.
    pub bucket_count: usize,

    /// Total duration the rolling window covers.
    /// Default: 10 seconds.
    pub window_duration: Duration,

    /// Failure rate threshold (0.0 to 1.0) to trip the circuit.
    /// Default: 0.5 (50%).
    pub failure_rate_threshold: f64,

    /// Minimum number of calls in the window before the failure rate
    /// is evaluated. Prevents tripping on low traffic. Default: 10.
    pub minimum_calls: u64,

    /// How long the circuit stays open before transitioning to HalfOpen.
    /// Default: 30 seconds.
    pub recovery_timeout: Duration,

    /// Number of probe calls allowed in HalfOpen state.
    /// If all succeed, the circuit closes. Default: 1.
    pub half_open_max_calls: usize,

    /// Optional timeout applied to each call. `None` means no timeout.
    /// Default: None.
    pub call_timeout: Option<Duration>,

    /// Maximum concurrent calls allowed (bulkhead). 0 means unlimited.
    /// Default: 0.
    pub max_concurrent_calls: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            bucket_count: 10,
            window_duration: Duration::from_secs(10),
            failure_rate_threshold: 0.5,
            minimum_calls: 10,
            recovery_timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
            call_timeout: None,
            max_concurrent_calls: 0,
        }
    }
}

impl CircuitBreakerConfig {
    /// Start building a configuration.
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }
}

/// Builder for `CircuitBreakerConfig`.
pub struct CircuitBreakerConfigBuilder {
    config: CircuitBreakerConfig,
}

impl CircuitBreakerConfigBuilder {
    fn new() -> Self {
        Self {
            config: CircuitBreakerConfig::default(),
        }
    }

    pub fn bucket_count(mut self, count: usize) -> Self {
        self.config.bucket_count = count.max(1);
        self
    }

    pub fn window_duration(mut self, duration: Duration) -> Self {
        self.config.window_duration = duration;
        self
    }

    pub fn failure_rate_threshold(mut self, threshold: f64) -> Self {
        self.config.failure_rate_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn minimum_calls(mut self, min: u64) -> Self {
        self.config.minimum_calls = min.max(1);
        self
    }

    pub fn recovery_timeout(mut self, timeout: Duration) -> Self {
        self.config.recovery_timeout = timeout;
        self
    }

    pub fn half_open_max_calls(mut self, max: usize) -> Self {
        self.config.half_open_max_calls = max.max(1);
        self
    }

    pub fn call_timeout(mut self, timeout: Duration) -> Self {
        self.config.call_timeout = Some(timeout);
        self
    }

    pub fn max_concurrent_calls(mut self, max: usize) -> Self {
        self.config.max_concurrent_calls = max;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> CircuitBreakerConfig {
        self.config
    }
}
