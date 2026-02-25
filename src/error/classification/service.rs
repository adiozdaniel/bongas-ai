/// Determines how the resilience layer responds to an error.
///
/// The circuit breaker, retry logic, and bulkhead all branch on this enum.
/// Domain modules never interact with resilience internals directly — they
/// classify their errors, and the infrastructure acts accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ErrorClassification {
    /// Transient failure — safe to retry, counts toward circuit breaker threshold.
    Transient,
    /// Permanent failure — do NOT retry, does NOT trip the circuit breaker.
    Permanent,
    /// Timeout — retryable with different backoff, counts toward circuit breaker.
    Timeout,
    /// Overload — downstream is overwhelmed. Do NOT retry immediately.
    /// Counts toward circuit breaker. Signals bulkhead to shed load.
    Overload,
    /// Degraded — operation partially succeeded or used fallback.
    /// May retry for full result, does NOT trip circuit breaker.
    Degraded,
    /// Partial failure — some items succeeded, some failed.
    /// Retry only failed items if possible.
    PartialFailure,
}

impl ErrorClassification {
    /// Returns true if this error type should be retried.
    #[inline]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::PartialFailure
        )
    }

    /// Returns true if this error should count toward circuit breaker threshold.
    #[inline]
    pub fn should_trip(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::Overload
        )
    }

    /// Returns true if retry should be delayed (backoff required).
    #[inline]
    pub fn requires_backoff(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Timeout | ErrorClassification::Overload
        )
    }
}

/// Classifies a domain error into a resilience-relevant category.
///
/// Every domain error type implements this trait so the circuit breaker and
/// retry logic can react without inspecting domain-specific variants.
pub trait ErrorClassifier {
    /// Classify this error for resilience decision-making.
    fn classify(&self) -> ErrorClassification;

    /// Returns true if this error should be retried.
    #[inline]
    fn is_retriable(&self) -> bool {
        self.classify().is_retriable()
    }

    /// Returns true if this error should count toward circuit breaker threshold.
    #[inline]
    fn should_trip(&self) -> bool {
        self.classify().should_trip()
    }

    /// Returns retry hints for this error.
    fn retry_hint(&self) -> crate::error::retry::RetryHint {
        use std::time::Duration;
        use crate::error::retry::RetryHint;
        
        match self.classify() {
            ErrorClassification::Transient => RetryHint::exponential(Duration::from_millis(100)),
            ErrorClassification::Timeout => RetryHint::exponential(Duration::from_millis(500)),
            ErrorClassification::Overload => RetryHint::exponential(Duration::from_secs(1)),
            ErrorClassification::Permanent => RetryHint::no_retry(),
            ErrorClassification::Degraded => RetryHint::exponential(Duration::from_millis(200)),
            ErrorClassification::PartialFailure => RetryHint::immediate(),
        }
    }

    /// Returns error context if available.
    fn context(&self) -> Option<&crate::error::context::ErrorContext> {
        None
    }
}
