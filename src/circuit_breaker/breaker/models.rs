use std::time::Duration;
use crate::circuit_breaker::observer::{CircuitBreakerId, CircuitState};
use crate::circuit_breaker::rolling_window::WindowSnapshot;

/// Health information for introspection/monitoring.
#[derive(Debug, Clone)]
pub struct CircuitBreakerHealth {
    pub id: CircuitBreakerId,
    pub state: CircuitState,
    pub metrics: WindowSnapshot,
    pub consecutive_failures: u32,
    pub time_until_recovery: Option<Duration>,
}
