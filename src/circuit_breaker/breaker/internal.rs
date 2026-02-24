use std::time::Duration;
use crate::error::ErrorClassification;

/// Context passed between phases of call execution.
pub struct CallContext {
    /// True if the call was made during HalfOpen state.
    pub in_half_open: bool,
}

/// Result of executing the operation.
pub enum ExecutionResult<T, E> {
    Success(T),
    Failure { error: E, classification: ErrorClassification },
    Timeout { timeout: Duration },
    SemaphoreClosed,
}
