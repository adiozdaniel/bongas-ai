//! Circuit breaker state models.

use crate::circuit_breaker::observer::CircuitState;

/// Result of attempting a state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TransitionResult {
    /// Transition was valid and applied.
    Transitioned { from: CircuitState, to: CircuitState },
    /// Transition was invalid and ignored (not allowed from current state).
    Rejected { current: CircuitState, attempted: CircuitState },
    /// Already in the target state (no-op).
    NoOp { current: CircuitState },
    /// Lost race with another thread (retry may succeed).
    RaceLost { current: CircuitState, attempted: CircuitState },
}

impl TransitionResult {
    /// Returns true if the transition succeeded.
    #[inline]
    pub fn succeeded(&self) -> bool {
        matches!(self, TransitionResult::Transitioned { .. })
    }

    /// Returns the resulting state after the transition attempt.
    #[inline]
    pub fn resulting_state(&self) -> CircuitState {
        match self {
            TransitionResult::Transitioned { to, .. } => *to,
            TransitionResult::Rejected { current, .. } => *current,
            TransitionResult::NoOp { current } => *current,
            TransitionResult::RaceLost { current, .. } => *current,
        }
    }
}
