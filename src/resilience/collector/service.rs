//! Metrics collector implementing the Observer pattern.
//!
//! Bridges circuit breaker events to the metrics registry.

use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;

use crate::circuit_breaker::{CircuitBreakerEvent, ResilienceObserver};
use crate::error::ErrorClassification;
use crate::resilience::registry::MetricsRegistry;
use crate::ml::training::online::service::{FeedbackWriter, FeedbackEvent};

/// Collects circuit breaker events and records them in the registry.
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

        if classification == ErrorClassification::Degraded {
            metrics.degraded_calls.increment();
        }
    }

    pub async fn record_ingestion_success(&self, activity_kind: &str, duration: Duration) {
        let label = format!("ingestion_{}", activity_kind);
        let metrics = self.registry.get_or_create(&label);
        metrics.successes.increment();
        metrics.latency.record_duration(duration);
    }

    pub async fn record_interaction_processed(&self, duration: Duration) {
        let metrics = self.registry.get_or_create("interaction_processed");
        metrics.successes.increment();
        metrics.latency.record_duration(duration);
    }
}

#[async_trait]
impl FeedbackWriter for ResilienceMetricsCollector {
    async fn write_batch(&self, events: &[FeedbackEvent]) -> anyhow::Result<()> {
        let metrics = self.registry.get_or_create("ml_feedback_batch");
        for _ in events {
            metrics.successes.increment();
        }
        Ok(())
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

            CircuitBreakerEvent::MetricsReset { .. } => {}

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
