//! Resource-Aware Circuit Breaker for the Training Pillar.
//! Proactively trips training if system resources are saturated.

use std::sync::Arc;
use tracing::warn;

use crate::resilience::collector::SystemHealthCollector;
use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerId, CircuitState};
use crate::error::classification::ErrorClassifier;
use crate::circuit_breaker::breaker::error::CircuitBreakerError;

/// Configuration for the Resource-Aware Circuit Breaker.
#[derive(Debug, Clone)]
pub struct ResourceBreakerConfig {
    pub cpu_threshold: f32,
    pub mem_threshold: f32,
}

impl Default for ResourceBreakerConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 85.0,
            mem_threshold: 90.0,
        }
    }
}

/// Resource-aware wrapper for the standard Circuit Breaker.
pub struct ResourceCircuitBreaker {
    pub(super) breaker: Arc<CircuitBreaker>,
    pub(super) health: Arc<SystemHealthCollector>,
    pub(super) config: ResourceBreakerConfig,
}

impl ResourceCircuitBreaker {
    pub fn new(
        _id: CircuitBreakerId,
        health: Arc<SystemHealthCollector>,
        config: ResourceBreakerConfig,
        base_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            breaker: base_breaker,
            health,
            config,
        }
    }

    /// Proactively check resources and trip the breaker if saturated.
    pub async fn check_resources(&self) -> bool {
        let health = self.health.get_health().await;
        
        let mut reason = None;

        if health.cpu_usage > self.config.cpu_threshold {
            reason = Some(format!("CPU Saturation: {:.2}%", health.cpu_usage));
        } else if health.mem_usage > self.config.mem_threshold {
            reason = Some(format!("Memory Saturation: {:.2}%", health.mem_usage));
        }

        if let Some(r) = reason {
            if self.breaker.state.state() == CircuitState::Closed {
                warn!(reason = %r, "Resource limit exceeded: Tripping training circuit");
            }
            return false;
        }

        true
    }

    /// Execute operation only if system is healthy.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: ErrorClassifier,
    {
        if !self.check_resources().await {
            return Err(CircuitBreakerError::Rejected {
                state: CircuitState::Open,
                retry_after: None,
            });
        }

        self.breaker.call(f).await
    }
}
