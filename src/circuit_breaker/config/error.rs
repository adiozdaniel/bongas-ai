//! Configuration validation error.

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
