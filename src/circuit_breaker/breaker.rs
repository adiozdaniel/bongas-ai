//! Generic circuit breaker core.
//!
//! Composes rolling window, state machine, configuration, and observer
//! into a single resilience primitive. Generic over the caller's error
//! type `E` — requires only `E: ErrorClassifier` to function.
//!
//! # Design Patterns
//! - **State Machine**: Explicit Closed → Open → HalfOpen transitions.
//! - **Strategy**: `ErrorClassifier` determines how errors affect the breaker.
//! - **Observer**: All events emitted through `ResilienceObserver`.
//! - **Builder**: Configuration via `CircuitBreakerConfig::builder()`.
//! - **Bulkhead**: Optional semaphore-based concurrency limiting.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

use crate::circuit_breaker::error::{ErrorClassification, ErrorClassifier};
use crate::circuit_breaker::observer::{
    CircuitBreakerEvent, CircuitBreakerId, CircuitState, ResilienceObserver, NoOpObserver,
};

use super::config::CircuitBreakerConfig;
use super::rolling_window::{RollingWindow, WindowSnapshot};
use super::state::{StateMachine, TransitionResult};

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

impl<E> CircuitBreakerError<E> {
    pub fn is_rejected(&self) -> bool {
        matches!(self, CircuitBreakerError::Rejected { .. })
    }

    pub fn is_execution_failed(&self) -> bool {
        matches!(self, CircuitBreakerError::ExecutionFailed { .. })
    }

    pub fn is_timed_out(&self) -> bool {
        matches!(self, CircuitBreakerError::TimedOut { .. })
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

/// Generic circuit breaker.
///
/// `E` is the caller's domain error type. It must implement `ErrorClassifier`
/// so the breaker knows which errors are transient (count toward tripping)
/// and which are permanent (ignored by the breaker).
pub struct CircuitBreaker {
    id: CircuitBreakerId,
    config: CircuitBreakerConfig,
    state_machine: Mutex<StateMachine>,
    rolling_window: Mutex<RollingWindow>,
    observer: Arc<dyn ResilienceObserver>,
    semaphore: Option<Arc<Semaphore>>,
    opened_at: Mutex<Option<Instant>>,
    half_open_calls: Mutex<usize>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given ID, config, and observer.
    pub fn new(
        id: CircuitBreakerId,
        config: CircuitBreakerConfig,
        observer: Arc<dyn ResilienceObserver>,
    ) -> Self {
        let semaphore = if config.max_concurrent_calls > 0 {
            Some(Arc::new(Semaphore::new(config.max_concurrent_calls)))
        } else {
            None
        };

        let rolling_window = RollingWindow::new(
            config.window_duration,
            config.bucket_count,
        );

        Self {
            id,
            config,
            state_machine: Mutex::new(StateMachine::new()),
            rolling_window: Mutex::new(rolling_window),
            observer,
            semaphore,
            opened_at: Mutex::new(None),
            half_open_calls: Mutex::new(0),
        }
    }

    /// Create a circuit breaker with default config and no observer.
    pub fn with_defaults(id: CircuitBreakerId) -> Self {
        Self::new(
            id,
            CircuitBreakerConfig::default(),
            Arc::new(NoOpObserver),
        )
    }

    /// Execute an async operation with circuit breaker protection.
    ///
    /// Returns `Ok(T)` if the operation succeeds, or `Err(CircuitBreakerError<E>)`
    /// if the circuit is open, the operation fails, or it times out.
    pub async fn call<F, Fut, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: ErrorClassifier,
    {
        // Check if we should transition from Open to HalfOpen
        self.check_recovery_timeout().await;

        // Check current state
        let current_state = self.state_machine.lock().await.state();

        match current_state {
            CircuitState::Open => {
                self.on_rejected().await;
                let retry_after = self.time_until_recovery().await;
                return Err(CircuitBreakerError::Rejected {
                    state: CircuitState::Open,
                    retry_after,
                });
            }
            CircuitState::HalfOpen => {
                let mut half_open = self.half_open_calls.lock().await;
                if *half_open >= self.config.half_open_max_calls {
                    self.on_rejected().await;
                    return Err(CircuitBreakerError::Rejected {
                        state: CircuitState::HalfOpen,
                        retry_after: None,
                    });
                }
                *half_open += 1;
            }
            CircuitState::Closed => {}
        }

        // Acquire semaphore permit if bulkhead is configured
        let _permit = match &self.semaphore {
            Some(sem) => Some(
                sem.clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| CircuitBreakerError::Rejected {
                        state: current_state,
                        retry_after: None,
                    })?,
            ),
            None => None,
        };

        // Execute with optional timeout
        let start = Instant::now();
        let result = match self.config.call_timeout {
            Some(timeout) => {
                match tokio::time::timeout(timeout, operation()).await {
                    Ok(result) => result,
                    Err(_) => {
                        let latency = start.elapsed();
                        self.on_timeout(latency).await;
                        return Err(CircuitBreakerError::TimedOut { timeout });
                    }
                }
            }
            None => operation().await,
        };

        let latency = start.elapsed();

        // Handle result
        match result {
            Ok(value) => {
                self.on_success(latency).await;
                Ok(value)
            }
            Err(error) => {
                let classification = error.classify();
                self.on_error(latency, classification).await;
                Err(CircuitBreakerError::ExecutionFailed {
                    source: error,
                    classification,
                    latency,
                })
            }
        }
    }

    /// Get a snapshot of the current rolling window.
    pub async fn snapshot(&self) -> WindowSnapshot {
        self.rolling_window.lock().await.snapshot()
    }

    /// Get the current circuit state.
    pub async fn state(&self) -> CircuitState {
        self.state_machine.lock().await.state()
    }

    /// Get the breaker ID.
    pub fn id(&self) -> &CircuitBreakerId {
        &self.id
    }

    /// Force the circuit closed and reset all counters.
    pub async fn reset(&self) {
        let mut sm = self.state_machine.lock().await;
        sm.force_closed();
        self.rolling_window.lock().await.reset();
        *self.opened_at.lock().await = None;
        *self.half_open_calls.lock().await = 0;

        self.observer.on_event(&CircuitBreakerEvent::MetricsReset {
            breaker_id: self.id.clone(),
        });
    }

    // ─── Internal ───────────────────────────────────────────────────────

    async fn on_success(&self, latency: Duration) {
        {
            let mut window = self.rolling_window.lock().await;
            window.record_success();
        }

        let current_state = self.state_machine.lock().await.state();

        if current_state == CircuitState::HalfOpen {
            let half_open = self.half_open_calls.lock().await;
            let snapshot = self.rolling_window.lock().await.snapshot();

            if snapshot.successes >= *half_open as u64 {
                self.transition_to(CircuitState::Closed).await;
                self.rolling_window.lock().await.reset();
                *self.half_open_calls.lock().await = 0;
                *self.opened_at.lock().await = None;
            }
        }

        self.observer.on_event(&CircuitBreakerEvent::CallSucceeded {
            breaker_id: self.id.clone(),
            latency,
        });
    }

    async fn on_error(&self, latency: Duration, classification: ErrorClassification) {
        // Only transient, timeout, and overload errors count
        let counts = matches!(
            classification,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::Overload
        );

        if counts {
            let mut window = self.rolling_window.lock().await;
            if classification == ErrorClassification::Timeout {
                window.record_timeout();
            } else {
                window.record_failure();
            }
        }

        self.observer.on_event(&CircuitBreakerEvent::CallFailed {
            breaker_id: self.id.clone(),
            latency,
            classification,
        });

        let current_state = self.state_machine.lock().await.state();

        match current_state {
            CircuitState::HalfOpen => {
                // Any countable failure in HalfOpen reopens the circuit
                if counts {
                    self.transition_to(CircuitState::Open).await;
                    *self.opened_at.lock().await = Some(Instant::now());
                    *self.half_open_calls.lock().await = 0;
                }
            }
            CircuitState::Closed => {
                if counts {
                    self.maybe_trip().await;
                }
            }
            CircuitState::Open => {}
        }
    }

    async fn on_timeout(&self, latency: Duration) {
        {
            let mut window = self.rolling_window.lock().await;
            window.record_timeout();
        }

        self.observer.on_event(&CircuitBreakerEvent::CallTimedOut {
            breaker_id: self.id.clone(),
            timeout: latency,
        });

        let current_state = self.state_machine.lock().await.state();

        match current_state {
            CircuitState::HalfOpen => {
                self.transition_to(CircuitState::Open).await;
                *self.opened_at.lock().await = Some(Instant::now());
                *self.half_open_calls.lock().await = 0;
            }
            CircuitState::Closed => {
                self.maybe_trip().await;
            }
            CircuitState::Open => {}
        }
    }

    async fn on_rejected(&self) {
        self.rolling_window.lock().await.record_rejection();

        self.observer.on_event(&CircuitBreakerEvent::CallRejected {
            breaker_id: self.id.clone(),
        });
    }

    /// Check if failure rate exceeds threshold and trip the circuit.
    async fn maybe_trip(&self) {
        let snapshot = self.rolling_window.lock().await.snapshot();

        if snapshot.total_calls >= self.config.minimum_calls
            && snapshot.failure_rate >= self.config.failure_rate_threshold
        {
            self.transition_to(CircuitState::Open).await;
            *self.opened_at.lock().await = Some(Instant::now());
        }
    }

    /// Check if recovery timeout has elapsed and transition to HalfOpen.
    async fn check_recovery_timeout(&self) {
        let current_state = self.state_machine.lock().await.state();
        if current_state != CircuitState::Open {
            return;
        }

        let opened_at = *self.opened_at.lock().await;
        if let Some(opened) = opened_at {
            if opened.elapsed() >= self.config.recovery_timeout {
                self.transition_to(CircuitState::HalfOpen).await;
                *self.half_open_calls.lock().await = 0;
            }
        }
    }

    /// How long until the circuit transitions to HalfOpen.
    async fn time_until_recovery(&self) -> Option<Duration> {
        let opened_at = *self.opened_at.lock().await;
        opened_at.map(|opened| {
            let elapsed = opened.elapsed();
            if elapsed >= self.config.recovery_timeout {
                Duration::ZERO
            } else {
                self.config.recovery_timeout - elapsed
            }
        })
    }

    /// Apply a state transition and notify observers.
    async fn transition_to(&self, target: CircuitState) {
        let result = self.state_machine.lock().await.transition_to(target);

        if let TransitionResult::Transitioned { from, to } = result {
            self.observer.on_event(&CircuitBreakerEvent::StateChanged {
                breaker_id: self.id.clone(),
                from,
                to,
            });
        }
    }
}
