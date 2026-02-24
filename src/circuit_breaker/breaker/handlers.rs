use std::time::Duration;
use crate::error::ErrorClassification;
use crate::circuit_breaker::observer::{CircuitBreakerEvent, CircuitState};
use crate::circuit_breaker::state::TransitionResult;
use super::CircuitBreaker;
use super::internal::{CallContext, ExecutionResult};

impl CircuitBreaker {
    // ─── Internal: Result Handling Phase ───────────────────────────────────

    pub(super) async fn handle_result<T, E>(
        &self,
        result: ExecutionResult<T, E>,
        latency: Duration,
        context: CallContext,
    ) -> Result<T, super::error::CircuitBreakerError<E>>
    {
        match result {
            ExecutionResult::Success(value) => {
                self.on_success(latency, context);
                Ok(value)
            }
            ExecutionResult::Failure { error, classification } => {
                self.on_failure(latency, classification, context);
                Err(super::error::CircuitBreakerError::ExecutionFailed {
                    source: error,
                    classification,
                    latency,
                })
            }
            ExecutionResult::Timeout { timeout } => {
                self.on_timeout(latency, context);
                Err(super::error::CircuitBreakerError::TimedOut { timeout })
            }
            ExecutionResult::SemaphoreClosed => {
                Err(super::error::CircuitBreakerError::Rejected {
                    state: self.state.state(),
                    retry_after: None,
                })
            }
        }
    }

    // ─── Internal: State Update Handlers ───────────────────────────────────

    pub(super) fn on_success(&self, latency: Duration, context: CallContext) {
        // Record in rolling window
        self.rolling_window.record_success(latency);

        // Check for slow call
        if let Some(threshold) = self.config.slow_call_duration() {
            if latency >= threshold {
                self.rolling_window.record_slow_call();
                self.observer.on_event(&CircuitBreakerEvent::SlowCall {
                    breaker_id: self.id.clone(),
                    latency,
                    threshold,
                });
            }
        }

        // Reset consecutive failures
        self.state.reset_consecutive_failures();

        // Handle HalfOpen success
        if context.in_half_open {
            let successes = self.state.record_half_open_success();

            // Check if we have enough successes to close
            if successes as usize >= self.config.half_open_max_calls() {
                let result = self.state.transition_to(CircuitState::Closed);
                if let TransitionResult::Transitioned { from, to } = result {
                    self.rolling_window.reset();
                    self.observer.on_event(&CircuitBreakerEvent::StateChanged {
                        breaker_id: self.id.clone(),
                        from,
                        to,
                    });
                }
            }
        }

        self.observer.on_event(&CircuitBreakerEvent::CallSucceeded {
            breaker_id: self.id.clone(),
            latency,
        });
    }

    pub(super) fn on_failure(&self, latency: Duration, classification: ErrorClassification, context: CallContext) {
        // Only transient, timeout, and overload errors count toward tripping
        let counts = matches!(
            classification,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::Overload
        );

        if counts {
            // Record in rolling window
            self.rolling_window.record_failure(latency);

            // Check for slow call
            if self.is_slow_call(latency) {
                self.rolling_window.record_slow_call();
            }

            // Update consecutive failure count
            let consecutive = self.state.record_failure();

            // Handle state transitions
            if context.in_half_open {
                // Any countable failure in HalfOpen reopens the circuit
                self.trip_circuit();
            } else if self.state.is_closed() {
                // Check if we should trip
                self.maybe_trip(consecutive);
            }
        }

        self.observer.on_event(&CircuitBreakerEvent::CallFailed {
            breaker_id: self.id.clone(),
            latency,
            classification,
        });
    }

    pub(super) fn on_timeout(&self, latency: Duration, context: CallContext) {
        self.rolling_window.record_timeout(latency);

        // Timeouts always count toward consecutive failures
        let consecutive = self.state.record_failure();

        self.observer.on_event(&CircuitBreakerEvent::CallTimedOut {
            breaker_id: self.id.clone(),
            timeout: latency,
        });

        if context.in_half_open {
            self.trip_circuit();
        } else if self.state.is_closed() {
            self.maybe_trip(consecutive);
        }
    }

    pub(super) fn on_rejected(&self) {
        self.rolling_window.record_rejection();

        self.observer.on_event(&CircuitBreakerEvent::CallRejected {
            breaker_id: self.id.clone(),
        });
    }

    // ─── Internal: Trip Logic ──────────────────────────────────────────────

    pub(super) fn maybe_trip(&self, consecutive_failures: u32) {
        // Check consecutive failure threshold first (faster)
        if let Some(threshold) = self.config.consecutive_failure_threshold() {
            if consecutive_failures >= threshold {
                self.trip_circuit();
                return;
            }
        }

        // Check rate-based thresholds
        let snapshot = self.rolling_window.snapshot();

        if !snapshot.has_minimum_calls(self.config.minimum_calls()) {
            return;
        }

        // Check failure rate threshold
        if snapshot.exceeds_failure_threshold(self.config.failure_rate_threshold()) {
            self.trip_circuit();
            return;
        }

        // Check slow call rate threshold
        if let Some(threshold) = self.config.slow_call_rate_threshold() {
            if snapshot.exceeds_slow_call_threshold(threshold) {
                self.trip_circuit();
            }
        }
    }

    pub(super) fn trip_circuit(&self) {
        let result = self.state.transition_to(CircuitState::Open);
        if let TransitionResult::Transitioned { from, to } = result {
            self.observer.on_event(&CircuitBreakerEvent::StateChanged {
                breaker_id: self.id.clone(),
                from,
                to,
            });
        }
    }

    #[inline]
    pub(super) fn is_slow_call(&self, latency: Duration) -> bool {
        self.config.slow_call_duration()
            .map(|threshold| latency >= threshold)
            .unwrap_or(false)
    }
}
