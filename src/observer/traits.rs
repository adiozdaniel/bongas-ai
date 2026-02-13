//! Observer trait definitions for the Composite Resilience Pattern.
//!
//! Observers are decoupled from emitters. The circuit breaker holds a
//! `Vec<Arc<dyn ResilienceObserver>>` and notifies all of them on every event.
//! Implementations decide what to do: record Prometheus metrics, log via
//! tracing, push to Kafka, or silently discard.

use crate::observer::event::CircuitBreakerEvent;

/// Receives events from resilience infrastructure.
///
/// Implementations must be `Send + Sync` because the circuit breaker may
/// be shared across threads via `Arc`.
///
/// The `on_event` method is intentionally synchronous — observers should
/// never block. If an observer needs async work (e.g. sending to Kafka),
/// it should buffer internally and flush on a background task.
pub trait ResilienceObserver: Send + Sync {
    /// Called by the circuit breaker on every state change or call result.
    fn on_event(&self, event: &CircuitBreakerEvent);
}

/// No-op observer that discards all events.
///
/// Used as the default when no observer is configured, avoiding
/// `Option<Arc<dyn ResilienceObserver>>` checks throughout the code.
pub struct NoOpObserver;

impl ResilienceObserver for NoOpObserver {
    fn on_event(&self, _event: &CircuitBreakerEvent) {}
}

/// Composite observer that fans out events to multiple observers.
///
/// Implements the Composite pattern — the circuit breaker sees a single
/// `ResilienceObserver`, but events reach all registered observers.
pub struct CompositeObserver {
    observers: Vec<std::sync::Arc<dyn ResilienceObserver>>,
}

impl CompositeObserver {
    pub fn new(observers: Vec<std::sync::Arc<dyn ResilienceObserver>>) -> Self {
        Self { observers }
    }
}

impl ResilienceObserver for CompositeObserver {
    fn on_event(&self, event: &CircuitBreakerEvent) {
        for observer in &self.observers {
            observer.on_event(event);
        }
    }
}

/// Logging observer that emits structured tracing events.
///
/// Provides out-of-the-box observability for any circuit breaker
/// without requiring Prometheus setup.
pub struct TracingObserver;

impl ResilienceObserver for TracingObserver {
    fn on_event(&self, event: &CircuitBreakerEvent) {
        let breaker = event.breaker_id().label();

        match event {
            CircuitBreakerEvent::CallSucceeded { latency, .. } => {
                tracing::debug!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    latency_ms = latency.as_millis() as u64,
                    "call succeeded"
                );
            }
            CircuitBreakerEvent::CallFailed { latency, classification, .. } => {
                tracing::warn!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    latency_ms = latency.as_millis() as u64,
                    classification = ?classification,
                    "call failed"
                );
            }
            CircuitBreakerEvent::CallRejected { .. } => {
                tracing::warn!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    "call rejected — circuit is open"
                );
            }
            CircuitBreakerEvent::StateChanged { from, to, .. } => {
                tracing::info!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    from = ?from,
                    to = ?to,
                    "state changed"
                );
            }
            CircuitBreakerEvent::CallTimedOut { timeout, .. } => {
                tracing::warn!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    timeout_ms = timeout.as_millis() as u64,
                    "call timed out"
                );
            }
            CircuitBreakerEvent::MetricsReset { .. } => {
                tracing::debug!(
                    target: "resilience::circuit_breaker",
                    breaker = %breaker,
                    "metrics reset"
                );
            }
        }
    }
}
