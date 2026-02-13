//! Circuit breaker configuration with Builder pattern and full validation.
//!
//! Provides ergonomic, validated configuration for circuit breakers with
//! Netflix Hystrix-style parameters including slow call detection.
//! All fields are private with accessor methods for proper encapsulation.

use std::time::Duration;

/// Validation error for circuit breaker configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValidationError {
    /// Window duration must be positive.
    InvalidWindowDuration,
    /// Bucket count must be at least 1.
    InvalidBucketCount,
    /// Failure rate threshold must be in range (0.0, 1.0].
    InvalidFailureRateThreshold,
    /// Slow call rate threshold must be in range (0.0, 1.0].
    InvalidSlowCallRateThreshold,
    /// Slow call duration must be positive when slow call detection is enabled.
    InvalidSlowCallDuration,
    /// Minimum calls must be at least 1.
    InvalidMinimumCalls,
    /// Recovery timeout must be positive.
    InvalidRecoveryTimeout,
    /// Half-open max calls must be at least 1.
    InvalidHalfOpenMaxCalls,
    /// Call timeout must be positive if specified.
    InvalidCallTimeout,
    /// Slow call duration must be less than call timeout.
    SlowCallDurationExceedsTimeout,
    /// Bucket duration would be zero (window too short for bucket count).
    BucketDurationTooShort,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWindowDuration => write!(f, "window duration must be positive"),
            Self::InvalidBucketCount => write!(f, "bucket count must be at least 1"),
            Self::InvalidFailureRateThreshold => {
                write!(f, "failure rate threshold must be in range (0.0, 1.0]")
            }
            Self::InvalidSlowCallRateThreshold => {
                write!(f, "slow call rate threshold must be in range (0.0, 1.0]")
            }
            Self::InvalidSlowCallDuration => {
                write!(f, "slow call duration must be positive when enabled")
            }
            Self::InvalidMinimumCalls => write!(f, "minimum calls must be at least 1"),
            Self::InvalidRecoveryTimeout => write!(f, "recovery timeout must be positive"),
            Self::InvalidHalfOpenMaxCalls => write!(f, "half-open max calls must be at least 1"),
            Self::InvalidCallTimeout => write!(f, "call timeout must be positive if specified"),
            Self::SlowCallDurationExceedsTimeout => {
                write!(f, "slow call duration must be less than call timeout")
            }
            Self::BucketDurationTooShort => {
                write!(f, "window duration too short for bucket count")
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Configuration for a circuit breaker instance.
///
/// All fields are private for encapsulation. Use `CircuitBreakerConfig::builder()`
/// for construction and accessor methods for reading values.
///
/// # Netflix Hystrix Defaults
/// - 10-second rolling window with 10 buckets
/// - 50% failure rate threshold to trip
/// - 5 minimum calls before evaluating
/// - 60-second recovery timeout
/// - 3 probe calls in half-open state
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of time buckets in the rolling window.
    bucket_count: usize,

    /// Total duration the rolling window covers.
    window_duration: Duration,

    /// Failure rate threshold (0.0 to 1.0) to trip the circuit.
    failure_rate_threshold: f64,

    /// Slow call rate threshold (0.0 to 1.0) to trip the circuit.
    /// If None, slow call detection is disabled.
    slow_call_rate_threshold: Option<f64>,

    /// Duration above which a call is considered slow.
    /// Required when slow_call_rate_threshold is set.
    slow_call_duration: Option<Duration>,

    /// Minimum number of calls in the window before evaluation.
    minimum_calls: u64,

    /// How long the circuit stays open before transitioning to HalfOpen.
    recovery_timeout: Duration,

    /// Number of probe calls allowed in HalfOpen state.
    half_open_max_calls: usize,

    /// Optional timeout applied to each call.
    call_timeout: Option<Duration>,

    /// Maximum concurrent calls allowed (bulkhead). 0 means unlimited.
    max_concurrent_calls: usize,

    /// Number of consecutive failures to trip (alternative to rate-based).
    /// If None, only rate-based tripping is used.
    consecutive_failure_threshold: Option<u32>,
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

/// Builder for `CircuitBreakerConfig` with validation.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfigBuilder {
    bucket_count: usize,
    window_duration: Duration,
    failure_rate_threshold: f64,
    slow_call_rate_threshold: Option<f64>,
    slow_call_duration: Option<Duration>,
    minimum_calls: u64,
    recovery_timeout: Duration,
    half_open_max_calls: usize,
    call_timeout: Option<Duration>,
    max_concurrent_calls: usize,
    consecutive_failure_threshold: Option<u32>,
}

impl CircuitBreakerConfigBuilder {
    fn new() -> Self {
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
