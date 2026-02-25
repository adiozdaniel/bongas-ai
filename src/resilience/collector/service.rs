//! Metrics collector implementing the Observer pattern.
//!
//! Bridges circuit breaker events to the metrics registry.
//!
//! # Netflix Resilience Features
//! - **Error Classification Tracking**: Records failures by classification type
//! - **Degraded Response Tracking**: Tracks degraded/partial failures separately

use std::sync::Arc;

use crate::circuit_breaker::{CircuitBreakerEvent, ResilienceObserver};
use crate::error::ErrorClassification;

use crate::resilience::registry::MetricsRegistry;

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

    /// Record a failure with its classification for detailed metrics.
    fn record_classified_failure(&self, breaker_id: &str, classification: ErrorClassification) {
        let metrics = self.registry.get_or_create(breaker_id);
        metrics.classifications.record(classification);

        // Also track degraded separately if applicable
        if classification == ErrorClassification::Degraded {
            metrics.degraded_calls.increment();
        }
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
                classification,
            } => {
                let label = breaker_id.label();
                let metrics = self.registry.get_or_create(&label);
                metrics.failures.increment();
                metrics.latency.record_duration(*latency);

                // Record classification for detailed breakdown
                self.record_classified_failure(&label, *classification);
            }

            CircuitBreakerEvent::CallTimedOut {
                breaker_id,
                timeout,
            } => {
                let label = breaker_id.label();
                let metrics = self.registry.get_or_create(&label);
                metrics.timeouts.increment();
                metrics.latency.record_duration(*timeout);

                // Timeouts are classified as Timeout
                self.record_classified_failure(&label, ErrorClassification::Timeout);
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

            CircuitBreakerEvent::SlowCall {
                breaker_id,
                latency,
                ..
            } => {
                let metrics = self.registry.get_or_create(&breaker_id.label());
                metrics.slow_calls.increment();
                metrics.latency.record_duration(*latency);
            }
        }
    }
}
