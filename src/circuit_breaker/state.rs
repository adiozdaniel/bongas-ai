//! Atomic circuit breaker state machine with lock-free transitions.
//!
//! Uses atomic operations for state storage, eliminating the need for
//! mutex locks during state checks. State transitions are atomic and
//! race-free using compare-and-swap operations.
//!
//! State transitions:
//! ```text
//! Closed ──(failure threshold exceeded)──▶ Open
//! Open ──(recovery timeout elapsed)──▶ HalfOpen
//! HalfOpen ──(probe succeeds)──▶ Closed
//! HalfOpen ──(probe fails)──▶ Open
//! ```

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::circuit_breaker::observer::CircuitState;

/// Internal representation of circuit state as u8 for atomic operations.
const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

/// Result of attempting a state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Unified state container holding all mutable circuit breaker state.
///
/// This struct consolidates state, timing, and counters to enable
/// atomic operations and reduce lock contention.
pub struct CircuitBreakerState {
    /// Current state (atomic for lock-free reads).
    state: AtomicU8,
    /// When the circuit was opened (for recovery timeout).
    /// Protected by mutex because Instant is not atomic.
    opened_at: Mutex<Option<Instant>>,
    /// Number of calls made in HalfOpen state.
    half_open_calls: AtomicU32,
    /// Number of successful calls in HalfOpen state.
    half_open_successes: AtomicU32,
    /// Consecutive failure count (for consecutive failure threshold).
    consecutive_failures: AtomicU32,
}

impl CircuitBreakerState {
    /// Creates a new state container in Closed state.
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_CLOSED),
            opened_at: Mutex::new(None),
            half_open_calls: AtomicU32::new(0),
            half_open_successes: AtomicU32::new(0),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    // ─── State Queries (Lock-Free) ─────────────────────────────────────────

    /// Returns the current state (lock-free).
    #[inline]
    pub fn state(&self) -> CircuitState {
        Self::u8_to_state(self.state.load(Ordering::Acquire))
    }

    /// Returns true if the circuit is closed.
    #[inline]
    pub fn is_closed(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_CLOSED
    }

    /// Returns true if the circuit is open.
    #[inline]
    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_OPEN
    }

    /// Returns true if the circuit is half-open.
    #[inline]
    pub fn is_half_open(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_HALF_OPEN
    }

    // ─── State Transitions ─────────────────────────────────────────────────

    /// Attempt to transition to a new state atomically.
    ///
    /// Only valid transitions are accepted:
    /// - Closed → Open
    /// - Open → HalfOpen
    /// - HalfOpen → Closed
    /// - HalfOpen → Open
    pub fn transition_to(&self, target: CircuitState) -> TransitionResult {
        let target_u8 = Self::state_to_u8(target);

        loop {
            let current_u8 = self.state.load(Ordering::Acquire);
            let current = Self::u8_to_state(current_u8);

            if current_u8 == target_u8 {
                return TransitionResult::NoOp { current };
            }

            if !Self::is_valid_transition(current, target) {
                return TransitionResult::Rejected {
                    current,
                    attempted: target,
                };
            }

            // Attempt atomic transition
            match self.state.compare_exchange(
                current_u8,
                target_u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Transition succeeded - update auxiliary state
                    self.on_transition(current, target);
                    return TransitionResult::Transitioned {
                        from: current,
                        to: target,
                    };
                }
                Err(actual) => {
                    // Lost race - check if we should retry or give up
                    let actual_state = Self::u8_to_state(actual);
                    if actual == target_u8 {
                        // Someone else made our transition
                        return TransitionResult::NoOp { current: actual_state };
                    }
                    // Retry the loop with new state
                    continue;
                }
            }
        }
    }

    /// Force the state to Closed, bypassing transition rules.
    /// Used only during reset operations.
    pub fn force_closed(&self) -> TransitionResult {
        let previous_u8 = self.state.swap(STATE_CLOSED, Ordering::AcqRel);
        let previous = Self::u8_to_state(previous_u8);

        // Reset all auxiliary state
        *self.lock_opened_at() = None;
        self.half_open_calls.store(0, Ordering::Release);
        self.half_open_successes.store(0, Ordering::Release);
        self.consecutive_failures.store(0, Ordering::Release);

        TransitionResult::Transitioned {
            from: previous,
            to: CircuitState::Closed,
        }
    }

    // ─── HalfOpen State Management ─────────────────────────────────────────

    /// Try to acquire a HalfOpen call slot.
    /// Returns true if a slot was acquired, false if limit reached.
    #[inline]
    pub fn try_acquire_half_open_slot(&self, max_calls: usize) -> bool {
        loop {
            let current = self.half_open_calls.load(Ordering::Acquire);
            if current as usize >= max_calls {
                return false;
            }

            match self.half_open_calls.compare_exchange(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(_) => continue, // Retry
            }
        }
    }

    /// Record a successful call in HalfOpen state.
    /// Returns the new success count.
    #[inline]
    pub fn record_half_open_success(&self) -> u32 {
        self.half_open_successes.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Get the number of successful HalfOpen calls.
    #[inline]
    pub fn half_open_successes(&self) -> u32 {
        self.half_open_successes.load(Ordering::Acquire)
    }

    /// Get the number of HalfOpen calls made.
    #[inline]
    pub fn half_open_calls(&self) -> u32 {
        self.half_open_calls.load(Ordering::Acquire)
    }

    // ─── Consecutive Failure Tracking ──────────────────────────────────────

    /// Record a failure, incrementing consecutive count.
    /// Returns the new consecutive failure count.
    #[inline]
    pub fn record_failure(&self) -> u32 {
        self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Reset consecutive failure count (on success).
    #[inline]
    pub fn reset_consecutive_failures(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
    }

    /// Get the current consecutive failure count.
    #[inline]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Acquire)
    }

    // ─── Timing ────────────────────────────────────────────────────────────

    /// Check if recovery timeout has elapsed.
    pub fn recovery_timeout_elapsed(&self, recovery_timeout: Duration) -> bool {
        let opened_at = *self.lock_opened_at();
        match opened_at {
            Some(instant) => instant.elapsed() >= recovery_timeout,
            None => false,
        }
    }

    /// Get time until recovery (None if not open or already elapsed).
    pub fn time_until_recovery(&self, recovery_timeout: Duration) -> Option<Duration> {
        let opened_at = *self.lock_opened_at();
        opened_at.map(|instant| {
            let elapsed = instant.elapsed();
            if elapsed >= recovery_timeout {
                Duration::ZERO
            } else {
                recovery_timeout - elapsed
            }
        })
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    /// Called after a successful transition to update auxiliary state.
    fn on_transition(&self, from: CircuitState, to: CircuitState) {
        match (from, to) {
            (_, CircuitState::Open) => {
                // Entering Open state - record when
                *self.lock_opened_at() = Some(Instant::now());
                self.half_open_calls.store(0, Ordering::Release);
                self.half_open_successes.store(0, Ordering::Release);
            }
            (CircuitState::Open, CircuitState::HalfOpen) => {
                // Entering HalfOpen - reset counters
                self.half_open_calls.store(0, Ordering::Release);
                self.half_open_successes.store(0, Ordering::Release);
            }
            (CircuitState::HalfOpen, CircuitState::Closed) => {
                // Closing circuit - reset all
                *self.lock_opened_at() = None;
                self.half_open_calls.store(0, Ordering::Release);
                self.half_open_successes.store(0, Ordering::Release);
                self.consecutive_failures.store(0, Ordering::Release);
            }
            _ => {}
        }
    }

    #[inline]
    fn state_to_u8(state: CircuitState) -> u8 {
        match state {
            CircuitState::Closed => STATE_CLOSED,
            CircuitState::Open => STATE_OPEN,
            CircuitState::HalfOpen => STATE_HALF_OPEN,
        }
    }

    #[inline]
    fn u8_to_state(value: u8) -> CircuitState {
        match value {
            STATE_CLOSED => CircuitState::Closed,
            STATE_OPEN => CircuitState::Open,
            STATE_HALF_OPEN => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Default to closed for safety
        }
    }

    #[inline]
    fn is_valid_transition(from: CircuitState, to: CircuitState) -> bool {
        matches!(
            (from, to),
            (CircuitState::Closed, CircuitState::Open)
                | (CircuitState::Open, CircuitState::HalfOpen)
                | (CircuitState::HalfOpen, CircuitState::Closed)
                | (CircuitState::HalfOpen, CircuitState::Open)
        )
    }

    /// Acquire lock on opened_at, recovering from poison if necessary.
    #[inline]
    fn lock_opened_at(&self) -> MutexGuard<'_, Option<Instant>> {
        self.opened_at.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self::new()
    }
}

// CircuitBreakerState is Send + Sync because:
// - AtomicU8 and AtomicU32 are Send + Sync
// - Mutex<Option<Instant>> is Send + Sync
