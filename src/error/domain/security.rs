use std::time::Duration;
use thiserror::Error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};
use crate::error::retry::RetryHint;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("license invalid: {0}")]
    LicenseInvalid(String),
    #[error("license revoked: {0}")]
    LicenseRevoked(String),
    #[error("hardware mismatch: {0}")]
    HardwareMismatch(String),
    #[error("binary tampered: {0}")]
    BinaryTampered(String),
    #[error("debugger detected")]
    DebuggerDetected,
    #[error("analysis tool detected: {0}")]
    AnalysisToolDetected(String),
    #[error("server validation failed: {reason}")]
    ServerValidationFailed { reason: String },
    #[error("revocation check failed: {reason}")]
    RevocationCheckFailed { reason: String },
    #[error("hardware fingerprint failed: {reason}")]
    HardwareFingerprintFailed { reason: String },
    #[error("server timeout after {timeout_ms}ms")]
    ServerTimeout { timeout_ms: u64 },
    #[error("heartbeat timeout after {timeout_ms}ms")]
    HeartbeatTimeout { timeout_ms: u64 },
    #[error("circuit breaker open for security layer")]
    CircuitOpen,
    #[error("validation overloaded (queue depth {queue_depth})")]
    ValidationOverloaded { queue_depth: usize },
    #[error("degraded security check: {layer} - {reason}")]
    Degraded { layer: String, reason: String },
    #[error("fallback used for security layer: {layer} - {reason}")]
    FallbackUsed { layer: String, reason: String },
}

impl ErrorClassifier for SecurityError {
    fn classify(&self) -> ErrorClassification {
        match self {
            SecurityError::LicenseInvalid(_)
            | SecurityError::LicenseRevoked(_)
            | SecurityError::HardwareMismatch(_)
            | SecurityError::BinaryTampered(_)
            | SecurityError::DebuggerDetected
            | SecurityError::AnalysisToolDetected(_) => ErrorClassification::Permanent,
            SecurityError::ServerValidationFailed { .. }
            | SecurityError::RevocationCheckFailed { .. }
            | SecurityError::HardwareFingerprintFailed { .. } => ErrorClassification::Transient,
            SecurityError::ServerTimeout { .. }
            | SecurityError::HeartbeatTimeout { .. } => ErrorClassification::Timeout,
            SecurityError::CircuitOpen
            | SecurityError::ValidationOverloaded { .. } => ErrorClassification::Overload,
            SecurityError::Degraded { .. }
            | SecurityError::FallbackUsed { .. } => ErrorClassification::Degraded,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            SecurityError::LicenseInvalid(_)
            | SecurityError::LicenseRevoked(_)
            | SecurityError::HardwareMismatch(_)
            | SecurityError::BinaryTampered(_)
            | SecurityError::DebuggerDetected
            | SecurityError::AnalysisToolDetected(_) => RetryHint::no_retry(),
            SecurityError::ServerValidationFailed { .. }
            | SecurityError::RevocationCheckFailed { .. }
            | SecurityError::HardwareFingerprintFailed { .. } => {
                RetryHint::exponential(Duration::from_millis(500))
            }
            SecurityError::ServerTimeout { .. }
            | SecurityError::HeartbeatTimeout { .. } => {
                RetryHint::exponential(Duration::from_secs(1))
            }
            SecurityError::CircuitOpen
            | SecurityError::ValidationOverloaded { .. } => {
                RetryHint::exponential(Duration::from_secs(2))
            }
            SecurityError::Degraded { .. }
            | SecurityError::FallbackUsed { .. } => RetryHint::immediate(),
        }
    }
}
