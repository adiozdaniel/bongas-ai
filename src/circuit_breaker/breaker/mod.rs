//! Production-grade circuit breaker implementation.

use std::future::Future;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

use crate::error::ErrorClassifier;
use crate::circuit_breaker::observer::{
    CircuitBreakerEvent, CircuitBreakerId, CircuitState, ResilienceObserver, NoOpObserver,
};

use super::config::CircuitBreakerConfig;
use super::rolling_window::{RollingWindow, WindowSnapshot};
use super::state::{CircuitBreakerState, TransitionResult};

pub mod error;
pub mod handlers;
pub mod internal;
pub mod models;

pub use self::error::CircuitBreakerError;
pub use self::models::CircuitBreakerHealth;
use self::internal::{CallContext, ExecutionResult};

/// Production-grade circuit breaker.
pub struct CircuitBreaker {
    pub(crate) id: CircuitBreakerId,
    pub(crate) config: CircuitBreakerConfig,
    pub(crate) state: CircuitBreakerState,
    pub(crate) rolling_window: RollingWindow,
    pub(crate) observer: Arc<dyn ResilienceObserver>,
    pub(crate) semaphore: Option<Arc<Semaphore>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given ID, config, and observer.
    pub fn new(
        id: CircuitBreakerId,
        config: CircuitBreakerConfig,
        observer: Arc<dyn ResilienceObserver>,
    ) -> Self {
        let semaphore = if config.max_concurrent_calls() > 0 {
            Some(Arc::new(Semaphore::new(config.max_concurrent_calls())))
        } else {
            None
        };

        let rolling_window = RollingWindow::new(
            config.window_duration(),
            config.bucket_count(),
        );

        Self {
            id,
            config,
            state: CircuitBreakerState::new(),
            rolling_window,
            observer,
            semaphore,
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
    pub async fn call<F, Fut, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: ErrorClassifier,
    {
        // Phase 1: Check state and acquire permission
        let call_context = self.acquire_permission().await?;

        // Phase 2: Execute with optional timeout
        let start = Instant::now();
        let result = self.execute_operation(operation, start).await;
        let latency = start.elapsed();

        // Phase 3: Handle result
        self.handle_result(result, latency, call_context).await
    }

    /// Get a snapshot of the current rolling window.
    #[inline]
    pub fn snapshot(&self) -> WindowSnapshot {
        self.rolling_window.snapshot()
    }

    /// Get the current circuit state.
    #[inline]
    pub fn current_state(&self) -> CircuitState {
        self.state.state()
    }

    /// Get the breaker ID.
    #[inline]
    pub fn id(&self) -> &CircuitBreakerId {
        &self.id
    }

    /// Get health information for monitoring/introspection.
    pub fn health(&self) -> CircuitBreakerHealth {
        CircuitBreakerHealth {
            id: self.id.clone(),
            state: self.state.state(),
            metrics: self.rolling_window.snapshot(),
            consecutive_failures: self.state.consecutive_failures(),
            time_until_recovery: self.state.time_until_recovery(self.config.recovery_timeout()),
        }
    }

    /// Force the circuit closed and reset all counters.
    pub fn reset(&self) {
        self.state.force_closed();
        self.rolling_window.reset();

        self.observer.on_event(&CircuitBreakerEvent::MetricsReset {
            breaker_id: self.id.clone(),
        });
    }

    // ─── Internal: Permission Phase ────────────────────────────────────────

    async fn acquire_permission<E>(&self) -> Result<CallContext, CircuitBreakerError<E>> {
        self.check_recovery_timeout();

        let current_state = self.state.state();

        match current_state {
            CircuitState::Open => {
                self.on_rejected();
                let retry_after = self.state.time_until_recovery(self.config.recovery_timeout());
                Err(CircuitBreakerError::Rejected {
                    state: CircuitState::Open,
                    retry_after,
                })
            }
            CircuitState::HalfOpen => {
                if !self.state.try_acquire_half_open_slot(self.config.half_open_max_calls()) {
                    self.on_rejected();
                    return Err(CircuitBreakerError::Rejected {
                        state: CircuitState::HalfOpen,
                        retry_after: None,
                    });
                }
                Ok(CallContext { in_half_open: true })
            }
            CircuitState::Closed => {
                Ok(CallContext { in_half_open: false })
            }
        }
    }

    fn check_recovery_timeout(&self) {
        if self.state.is_open()
            && self.state.recovery_timeout_elapsed(self.config.recovery_timeout())
        {
            let result = self.state.transition_to(CircuitState::HalfOpen);
            if let TransitionResult::Transitioned { from, to } = result {
                self.observer.on_event(&CircuitBreakerEvent::StateChanged {
                    breaker_id: self.id.clone(),
                    from,
                    to,
                });
            }
        }
    }

    // ─── Internal: Execution Phase ─────────────────────────────────────────

    async fn execute_operation<F, Fut, T, E>(
        &self,
        operation: F,
        _start: Instant,
    ) -> ExecutionResult<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: ErrorClassifier,
    {
        let _permit = match &self.semaphore {
            Some(sem) => {
                match sem.clone().acquire_owned().await {
                    Ok(permit) => Some(permit),
                    Err(_) => return ExecutionResult::SemaphoreClosed,
                }
            }
            None => None,
        };

        match self.config.call_timeout() {
            Some(timeout) => {
                match tokio::time::timeout(timeout, operation()).await {
                    Ok(result) => match result {
                        Ok(value) => ExecutionResult::Success(value),
                        Err(error) => {
                            let classification = error.classify();
                            ExecutionResult::Failure { error, classification }
                        }
                    },
                    Err(_) => ExecutionResult::Timeout { timeout },
                }
            }
            None => {
                match operation().await {
                    Ok(value) => ExecutionResult::Success(value),
                    Err(error) => {
                        let classification = error.classify();
                        ExecutionResult::Failure { error, classification }
                    }
                }
            }
        }
    }
}
