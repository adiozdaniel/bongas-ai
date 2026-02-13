//! Event types emitted by resilience infrastructure.
//!
//! Every event carries enough context for observers to record metrics,
//! emit logs, or trigger alerts without reaching back into the emitter.

use std::time::Duration;
use crate::circuit_breaker::error::ErrorClassification;

/// Identifies which circuit breaker emitted the event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CircuitBreakerId {
    /// Component name (e.g. "redis", "postgres", "kafka").
    pub component: &'static str,
    /// Optional sub-identifier (e.g. "user_cache", "content_cache").
    pub instance: Option<&'static str>,
}

impl CircuitBreakerId {
    pub fn new(component: &'static str) -> Self {
        Self { component, instance: None }
    }

    pub fn with_instance(component: &'static str, instance: &'static str) -> Self {
        Self { component, instance: Some(instance) }
    }

    /// Returns a label suitable for Prometheus metrics.
    pub fn label(&self) -> String {
        match self.instance {
            Some(inst) => format!("{}_{}", self.component, inst),
            None => self.component.to_string(),
        }
    }
}

/// Circuit breaker state for event reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Events emitted by the circuit breaker.
#[derive(Debug, Clone)]
pub enum CircuitBreakerEvent {
    /// A call was executed successfully.
    CallSucceeded {
        breaker_id: CircuitBreakerId,
        latency: Duration,
    },

    /// A call was executed but failed.
    CallFailed {
        breaker_id: CircuitBreakerId,
        latency: Duration,
        classification: ErrorClassification,
    },

    /// A call was rejected because the circuit is open.
    CallRejected {
        breaker_id: CircuitBreakerId,
    },

    /// The circuit breaker changed state.
    StateChanged {
        breaker_id: CircuitBreakerId,
        from: CircuitState,
        to: CircuitState,
    },

    /// A call timed out.
    CallTimedOut {
        breaker_id: CircuitBreakerId,
        timeout: Duration,
    },

    /// The rolling window was reset (e.g. after closing the circuit).
    MetricsReset {
        breaker_id: CircuitBreakerId,
    },
}

impl CircuitBreakerEvent {
    /// Returns the breaker ID for any event variant.
    pub fn breaker_id(&self) -> &CircuitBreakerId {
        match self {
            CircuitBreakerEvent::CallSucceeded { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::CallFailed { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::CallRejected { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::StateChanged { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::CallTimedOut { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::MetricsReset { breaker_id, .. } => breaker_id,
        }
    }
}
