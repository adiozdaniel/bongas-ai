//! Metrics collector implementing the Observer pattern.
//!
//! Bridges circuit breaker events to the metrics registry.

use std::sync::Arc;

use crate::circuit_breaker::{CircuitBreakerEvent, ResilienceObserver};

use super::registry::MetricsRegistry;

/// Collects circuit breaker events and records them in the registry.
///
/// Implements `ResilienceObserver` to receive events from circuit breakers.
pub struct ResilienceMetricsCollector {
    registry: Arc<MetricsRegistry>,
}

impl ResilienceMetricsCollector {
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &MetricsRegistry {
        &self.registry
    }
}

impl ResilienceObserver for ResilienceMetricsCollector {
    fn on_event(&self, event: &CircuitBreakerEvent) {
        match event {
            CircuitBreakerEvent::CallSucceeded {
                breaker_id,
                latency,
            } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.successes.increment();
                metrics.latency.record_duration(*latency);
            }

            CircuitBreakerEvent::CallFailed {
                breaker_id,
                latency,
                ..
            } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.failures.increment();
                metrics.latency.record_duration(*latency);
            }

            CircuitBreakerEvent::CallTimedOut {
                breaker_id,
                timeout,
            } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.timeouts.increment();
                metrics.latency.record_duration(*timeout);
            }

            CircuitBreakerEvent::CallRejected { breaker_id } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.rejections.increment();
            }

            CircuitBreakerEvent::StateChanged {
                breaker_id,
                to,
                ..
            } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.update_state(*to);
            }

            CircuitBreakerEvent::MetricsReset { .. } => {
                // No action needed - registry tracks independently
            }
        }
    }
}
