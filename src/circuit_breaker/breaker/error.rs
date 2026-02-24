use std::fmt;
use std::time::Duration;
use crate::error::ErrorClassification;
use crate::circuit_breaker::observer::CircuitState;

/// Error returned by the circuit breaker to callers.
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// The circuit is open — call was never executed.
    Rejected {
        state: CircuitState,
        retry_after: Option<Duration>,
    },
    /// The call was executed but the operation failed.
    ExecutionFailed {
        source: E,
        classification: ErrorClassification,
        latency: Duration,
    },
    /// The call exceeded the configured timeout.
    TimedOut {
        timeout: Duration,
    },
}

impl<E: fmt::Display> fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { state, retry_after } => {
                write!(f, "circuit breaker rejected call (state: {:?}", state)?;
                if let Some(retry) = retry_after {
                    write!(f, ", retry after: {:?}", retry)?;
                }
                write!(f, ")")
            }
            Self::ExecutionFailed { source, classification, latency } => {
                write!(
                    f,
                    "execution failed: {} (classification: {:?}, latency: {:?})",
                    source, classification, latency
                )
            }
            Self::TimedOut { timeout } => {
                write!(f, "call timed out after {:?}", timeout)
            }
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None // E may not implement Error, so we can't return source
    }
}

impl<E> CircuitBreakerError<E> {
    #[inline]
    pub fn is_rejected(&self) -> bool {
        matches!(self, CircuitBreakerError::Rejected { .. })
    }

    #[inline]
    pub fn is_execution_failed(&self) -> bool {
        matches!(self, CircuitBreakerError::ExecutionFailed { .. })
    }

    #[inline]
    pub fn is_timed_out(&self) -> bool {
        matches!(self, CircuitBreakerError::TimedOut { .. })
    }

    /// Returns the inner error if this is an ExecutionFailed variant.
    pub fn into_inner(self) -> Option<E> {
        match self {
            CircuitBreakerError::ExecutionFailed { source, .. } => Some(source),
            _ => None,
        }
    }

    /// Map the inner error to a different type.
    pub fn map_err<F, U>(self, f: F) -> CircuitBreakerError<U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            CircuitBreakerError::Rejected { state, retry_after } => {
                CircuitBreakerError::Rejected { state, retry_after }
            }
            CircuitBreakerError::ExecutionFailed { source, classification, latency } => {
                CircuitBreakerError::ExecutionFailed {
                    source: f(source),
                    classification,
                    latency,
                }
            }
            CircuitBreakerError::TimedOut { timeout } => {
                CircuitBreakerError::TimedOut { timeout }
            }
        }
    }
}
