//! Event types emitted by resilience infrastructure.
//!
//! Every event carries enough context for observers to record metrics,
//! emit logs, or trigger alerts without reaching back into the emitter.

use std::time::Duration;
use crate::error::ErrorClassification;

/// Identifies which circuit breaker emitted the event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CircuitBreakerId {
    /// Component name (e.g. "redis", "postgres", "kafka").
    component: &'static str,
    /// Optional sub-identifier (e.g. "user_cache", "content_cache").
    instance: Option<&'static str>,
}

impl CircuitBreakerId {
    /// Create a new ID with just a component name.
    pub const fn new(component: &'static str) -> Self {
        Self { component, instance: None }
    }

    /// Create a new ID with component and instance names.
    pub const fn with_instance(component: &'static str, instance: &'static str) -> Self {
        Self { component, instance: Some(instance) }
    }

    /// Get the component name.
    #[inline]
    pub fn component(&self) -> &'static str {
        self.component
    }

    /// Get the instance name, if any.
    #[inline]
    pub fn instance(&self) -> Option<&'static str> {
        self.instance
    }

    /// Returns a label suitable for Prometheus metrics.
    pub fn label(&self) -> String {
        match self.instance {
            Some(inst) => format!("{}_{}", self.component, inst),
            None => self.component.to_string(),
        }
    }
}

impl std::fmt::Display for CircuitBreakerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instance {
            Some(inst) => write!(f, "{}:{}", self.component, inst),
            None => write!(f, "{}", self.component),
        }
    }
}

/// Circuit breaker state for event reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitState {
    /// Circuit is closed - calls flow through normally.
    Closed,
    /// Circuit is open - calls are rejected immediately.
    Open,
    /// Circuit is testing - limited calls allowed to probe recovery.
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// Events emitted by the circuit breaker.
///
/// All events include the breaker ID for correlation in multi-breaker systems.
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

    /// A call was rejected because the circuit is open or at capacity.
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

    /// A call was successful but exceeded the slow call threshold.
    SlowCall {
        breaker_id: CircuitBreakerId,
        latency: Duration,
        threshold: Duration,
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
            CircuitBreakerEvent::SlowCall { breaker_id, .. } => breaker_id,
            CircuitBreakerEvent::MetricsReset { breaker_id, .. } => breaker_id,
        }
    }

    /// Returns the event type as a string (for metrics labels).
    pub fn event_type(&self) -> &'static str {
        match self {
            CircuitBreakerEvent::CallSucceeded { .. } => "call_succeeded",
            CircuitBreakerEvent::CallFailed { .. } => "call_failed",
            CircuitBreakerEvent::CallRejected { .. } => "call_rejected",
            CircuitBreakerEvent::StateChanged { .. } => "state_changed",
            CircuitBreakerEvent::CallTimedOut { .. } => "call_timed_out",
            CircuitBreakerEvent::SlowCall { .. } => "slow_call",
            CircuitBreakerEvent::MetricsReset { .. } => "metrics_reset",
        }
    }

    /// Returns true if this event indicates a problem (failure, timeout, rejection).
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            CircuitBreakerEvent::CallFailed { .. }
                | CircuitBreakerEvent::CallRejected { .. }
                | CircuitBreakerEvent::CallTimedOut { .. }
                | CircuitBreakerEvent::SlowCall { .. }
        )
    }

    /// Returns the latency if this event type has one.
    pub fn latency(&self) -> Option<Duration> {
        match self {
            CircuitBreakerEvent::CallSucceeded { latency, .. } => Some(*latency),
            CircuitBreakerEvent::CallFailed { latency, .. } => Some(*latency),
            CircuitBreakerEvent::CallTimedOut { timeout, .. } => Some(*timeout),
            CircuitBreakerEvent::SlowCall { latency, .. } => Some(*latency),
            _ => None,
        }
    }
}
