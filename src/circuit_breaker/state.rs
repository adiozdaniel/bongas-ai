//! Circuit breaker state machine with explicit transitions.
//!
//! Models the three states of a circuit breaker (Closed, Open, HalfOpen)
//! and enforces valid transitions. Invalid transitions are rejected,
//! preventing the breaker from entering an inconsistent state.
//!
//! State transitions:
//! ```text
//! Closed ──(failure threshold exceeded)──▶ Open
//! Open ──(recovery timeout elapsed)──▶ HalfOpen
//! HalfOpen ──(probe succeeds)──▶ Closed
//! HalfOpen ──(probe fails)──▶ Open
//! ```

use crate::circuit_breaker::observer::CircuitState;

/// Result of attempting a state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition was valid and applied.
    Transitioned { from: CircuitState, to: CircuitState },
    /// Transition was invalid and ignored.
    Rejected { current: CircuitState, attempted: CircuitState },
    /// Already in the target state.
    NoOp { current: CircuitState },
}

/// State machine that enforces valid circuit breaker transitions.
pub struct StateMachine {
    current: CircuitState,
}

impl StateMachine {
    /// Creates a new state machine in the Closed state.
    pub fn new() -> Self {
        Self {
            current: CircuitState::Closed,
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> CircuitState {
        self.current
    }

    /// Attempt to transition to a new state.
    ///
    /// Only valid transitions are accepted:
    /// - Closed → Open
    /// - Open → HalfOpen
    /// - HalfOpen → Closed
    /// - HalfOpen → Open
    pub fn transition_to(&mut self, target: CircuitState) -> TransitionResult {
        if self.current == target {
            return TransitionResult::NoOp { current: self.current };
        }

        if self.is_valid_transition(target) {
            let from = self.current;
            self.current = target;
            TransitionResult::Transitioned { from, to: target }
        } else {
            TransitionResult::Rejected {
                current: self.current,
                attempted: target,
            }
        }
    }

    /// Check if a transition from current state to target is valid.
    fn is_valid_transition(&self, target: CircuitState) -> bool {
        matches!(
            (self.current, target),
            (CircuitState::Closed, CircuitState::Open)
                | (CircuitState::Open, CircuitState::HalfOpen)
                | (CircuitState::HalfOpen, CircuitState::Closed)
                | (CircuitState::HalfOpen, CircuitState::Open)
        )
    }

    /// Force the state machine into Closed, bypassing transition rules.
    /// Used only during reset operations.
    pub fn force_closed(&mut self) -> TransitionResult {
        let from = self.current;
        self.current = CircuitState::Closed;
        TransitionResult::Transitioned {
            from,
            to: CircuitState::Closed,
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
