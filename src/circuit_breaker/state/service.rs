//! Atomic circuit breaker state machine implementation.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::circuit_breaker::observer::CircuitState;
use super::models::TransitionResult;

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

/// Unified state container holding all mutable circuit breaker state.
pub struct CircuitBreakerState {
    state: AtomicU8,
    opened_at: Mutex<Option<Instant>>,
    half_open_calls: AtomicU32,
    half_open_successes: AtomicU32,
    consecutive_failures: AtomicU32,
}

impl CircuitBreakerState {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_CLOSED),
            opened_at: Mutex::new(None),
            half_open_calls: AtomicU32::new(0),
            half_open_successes: AtomicU32::new(0),
            consecutive_failures: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn state(&self) -> CircuitState {
        Self::u8_to_state(self.state.load(Ordering::Acquire))
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_CLOSED
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_OPEN
    }

    #[inline]
    pub fn is_half_open(&self) -> bool {
        self.state.load(Ordering::Acquire) == STATE_HALF_OPEN
    }

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

            let mut opened_at_guard = self.lock_opened_at();

            let re_current_u8 = self.state.load(Ordering::Acquire);
            if re_current_u8 != current_u8 {
                continue;
            }

            match self.state.compare_exchange(
                current_u8,
                target_u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    match target {
                        CircuitState::Open => {
                            *opened_at_guard = Some(Instant::now());
                            self.half_open_calls.store(0, Ordering::Release);
                            self.half_open_successes.store(0, Ordering::Release);
                        }
                        CircuitState::HalfOpen => {
                            self.half_open_calls.store(0, Ordering::Release);
                            self.half_open_successes.store(0, Ordering::Release);
                        }
                        CircuitState::Closed => {
                            *opened_at_guard = None;
                            self.half_open_calls.store(0, Ordering::Release);
                            self.half_open_successes.store(0, Ordering::Release);
                            self.consecutive_failures.store(0, Ordering::Release);
                        }
                    }
                    
                    return TransitionResult::Transitioned {
                        from: current,
                        to: target,
                    };
                }
                Err(actual) => {
                    let actual_state = Self::u8_to_state(actual);
                    if actual == target_u8 {
                        return TransitionResult::NoOp { current: actual_state };
                    }
                    continue;
                }
            }
        }
    }

    pub fn force_closed(&self) -> TransitionResult {
        let previous_u8 = self.state.swap(STATE_CLOSED, Ordering::AcqRel);
        let previous = Self::u8_to_state(previous_u8);

        *self.lock_opened_at() = None;
        self.half_open_calls.store(0, Ordering::Release);
        self.half_open_successes.store(0, Ordering::Release);
        self.consecutive_failures.store(0, Ordering::Release);

        TransitionResult::Transitioned {
            from: previous,
            to: CircuitState::Closed,
        }
    }

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
                Err(_) => continue,
            }
        }
    }

    #[inline]
    pub fn record_half_open_success(&self) -> u32 {
        self.half_open_successes.fetch_add(1, Ordering::AcqRel) + 1
    }

    #[inline]
    pub fn half_open_successes(&self) -> u32 {
        self.half_open_successes.load(Ordering::Acquire)
    }

    #[inline]
    pub fn half_open_calls(&self) -> u32 {
        self.half_open_calls.load(Ordering::Acquire)
    }

    #[inline]
    pub fn record_failure(&self) -> u32 {
        self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1
    }

    #[inline]
    pub fn reset_consecutive_failures(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
    }

    #[inline]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Acquire)
    }

    pub fn recovery_timeout_elapsed(&self, recovery_timeout: Duration) -> bool {
        let opened_at = *self.lock_opened_at();
        match opened_at {
            Some(instant) => instant.elapsed() >= recovery_timeout,
            None => false,
        }
    }

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
            _ => CircuitState::Closed,
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
