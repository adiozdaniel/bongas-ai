//! Metrics exporters for various output formats.
//!
//! Implements the Strategy pattern — different exporters can be plugged
//! in to output metrics in different formats (Prometheus, JSON, etc.)
//!
//! # Netflix Resilience Features
//! - **Error Classification Export**: Breakdown by classification type
//! - **Degraded/Slow Call Metrics**: Exported separately for alerting

use crate::resilience::registry::MetricsRegistry;

/// Trait for exporting metrics to various formats.
///
/// Strategy pattern: implement this trait to add new export formats.
pub trait MetricsExporter: Send + Sync {
    /// Export metrics and return the formatted output.
    fn export(&self, registry: &MetricsRegistry) -> String;

    /// Content type for HTTP responses.
    fn content_type(&self) -> &'static str;
}

// ─── Prometheus Exporter ────────────────────────────────────────────────────

/// Exports metrics in Prometheus text exposition format.
pub struct PrometheusExporter {
    namespace: String,
}

impl PrometheusExporter {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    fn format_metric(&self, name: &str, labels: &str, value: f64) -> String {
        if labels.is_empty() {
            format!("{}_{} {}\n", self.namespace, name, value)
        } else {
            format!("{}_{}{{{}}} {}\n", self.namespace, name, labels, value)
        }
    }

    fn format_metadata(&self, name: &str, mtype: &str, help: &str) -> String {
        format!(
            "# HELP {}_{} {}\n# TYPE {}_{} {}\n",
            self.namespace, name, help, self.namespace, name, mtype
        )
    }
}

impl MetricsExporter for PrometheusExporter {
    fn export(&self, registry: &MetricsRegistry) -> String {
        let snapshot = registry.snapshot();
        let mut output = String::with_capacity(8192);

        // --- Metadata ---
        output.push_str(&self.format_metadata("calls_total", "counter", "Total number of calls recorded by the circuit breaker."));
        output.push_str(&self.format_metadata("successes_total", "counter", "Total number of successful calls."));
        output.push_str(&self.format_metadata("failures_total", "counter", "Total number of failed calls (excluding timeouts)."));
        output.push_str(&self.format_metadata("timeouts_total", "counter", "Total number of timed out calls."));
        output.push_str(&self.format_metadata("rejections_total", "counter", "Total number of calls rejected by circuit breaker or bulkhead."));
        output.push_str(&self.format_metadata("slow_calls_total", "counter", "Total number of calls that were successful but exceeded the slow call threshold."));
        output.push_str(&self.format_metadata("degraded_calls_total", "counter", "Total number of calls that resulted in a degraded response."));
        output.push_str(&self.format_metadata("failure_rate", "gauge", "Current failure rate as a fraction between 0 and 1."));
        output.push_str(&self.format_metadata("throughput", "gauge", "Current throughput in calls per second."));
        output.push_str(&self.format_metadata("latency_p50_ms", "gauge", "50th percentile latency in milliseconds."));
        output.push_str(&self.format_metadata("latency_p90_ms", "gauge", "90th percentile latency in milliseconds."));
        output.push_str(&self.format_metadata("latency_p95_ms", "gauge", "95th percentile latency in milliseconds."));
        output.push_str(&self.format_metadata("latency_p99_ms", "gauge", "99th percentile latency in milliseconds."));
        output.push_str(&self.format_metadata("latency_mean_ms", "gauge", "Mean latency in milliseconds."));
        output.push_str(&self.format_metadata("concurrent_calls", "gauge", "Number of calls currently in progress."));
        output.push_str(&self.format_metadata("state_duration_seconds", "gauge", "Time in seconds since the last state change."));
        output.push_str(&self.format_metadata("state", "gauge", "Current state of the circuit breaker (1 if in the labeled state)."));
        
        output.push_str(&self.format_metadata("errors_transient_total", "counter", "Total transient errors."));
        output.push_str(&self.format_metadata("errors_permanent_total", "counter", "Total permanent errors."));
        output.push_str(&self.format_metadata("errors_timeout_total", "counter", "Total timeout errors."));
        output.push_str(&self.format_metadata("errors_overload_total", "counter", "Total overload/rate-limit errors."));
        output.push_str(&self.format_metadata("errors_degraded_total", "counter", "Total degraded response errors."));
        output.push_str(&self.format_metadata("errors_partial_failure_total", "counter", "Total partial failure errors."));

        output.push_str(&self.format_metadata("aggregate_calls_total", "counter", "Total calls across all circuit breakers."));
        output.push_str(&self.format_metadata("aggregate_successes_total", "counter", "Total successes across all circuit breakers."));
        output.push_str(&self.format_metadata("aggregate_failures_total", "counter", "Total failures across all circuit breakers."));
        output.push_str(&self.format_metadata("aggregate_failure_rate", "gauge", "Aggregate failure rate across all circuit breakers."));
        output.push_str(&self.format_metadata("aggregate_throughput", "gauge", "Aggregate throughput across all circuit breakers."));

        // Per-breaker metrics
        for breaker in &snapshot.breakers {
            let labels = format!("breaker=\"{}\"", breaker.breaker_id);

            output.push_str(&self.format_metric("calls_total", &labels, breaker.total_calls as f64));
            output.push_str(&self.format_metric("successes_total", &labels, breaker.successes as f64));
            output.push_str(&self.format_metric("failures_total", &labels, breaker.failures as f64));
            output.push_str(&self.format_metric("timeouts_total", &labels, breaker.timeouts as f64));
            output.push_str(&self.format_metric("rejections_total", &labels, breaker.rejections as f64));
            output.push_str(&self.format_metric("slow_calls_total", &labels, breaker.slow_calls as f64));
            output.push_str(&self.format_metric("degraded_calls_total", &labels, breaker.degraded_calls as f64));
            output.push_str(&self.format_metric("failure_rate", &labels, breaker.failure_rate));
            output.push_str(&self.format_metric("throughput", &labels, breaker.throughput));
            output.push_str(&self.format_metric("latency_p50_ms", &labels, breaker.latency_p50_ms));
            output.push_str(&self.format_metric("latency_p90_ms", &labels, breaker.latency_p90_ms));
            output.push_str(&self.format_metric("latency_p95_ms", &labels, breaker.latency_p95_ms));
            output.push_str(&self.format_metric("latency_p99_ms", &labels, breaker.latency_p99_ms));
            output.push_str(&self.format_metric("latency_mean_ms", &labels, breaker.latency_mean_ms));
            output.push_str(&self.format_metric("concurrent_calls", &labels, breaker.concurrent_calls as f64));
            output.push_str(&self.format_metric("state_duration_seconds", &labels, breaker.state_duration_secs));

            let state_labels = format!("{},state=\"{}\"", labels, breaker.state);
            output.push_str(&self.format_metric("state", &state_labels, 1.0));

            let class = &breaker.classifications;
            output.push_str(&self.format_metric("errors_transient_total", &labels, class.transient as f64));
            output.push_str(&self.format_metric("errors_permanent_total", &labels, class.permanent as f64));
            output.push_str(&self.format_metric("errors_timeout_total", &labels, class.timeout as f64));
            output.push_str(&self.format_metric("errors_overload_total", &labels, class.overload as f64));
            output.push_str(&self.format_metric("errors_degraded_total", &labels, class.degraded as f64));
            output.push_str(&self.format_metric("errors_partial_failure_total", &labels, class.partial_failure as f64));
        }

        // Aggregate metrics
        output.push_str(&self.format_metric("aggregate_calls_total", "", snapshot.total_calls as f64));
        output.push_str(&self.format_metric("aggregate_successes_total", "", snapshot.total_successes as f64));
        output.push_str(&self.format_metric("aggregate_failures_total", "", snapshot.total_failures as f64));
        output.push_str(&self.format_metric("aggregate_timeouts_total", "", snapshot.total_timeouts as f64));
        output.push_str(&self.format_metric("aggregate_rejections_total", "", snapshot.total_rejections as f64));
        output.push_str(&self.format_metric("aggregate_slow_calls_total", "", snapshot.total_slow_calls as f64));
        output.push_str(&self.format_metric("aggregate_degraded_total", "", snapshot.total_degraded as f64));
        output.push_str(&self.format_metric("aggregate_failure_rate", "", snapshot.aggregate_failure_rate));
        output.push_str(&self.format_metric("aggregate_throughput", "", snapshot.aggregate_throughput));

        let agg = &snapshot.aggregate_classifications;
        output.push_str(&self.format_metric("aggregate_errors_transient_total", "", agg.transient as f64));
        output.push_str(&self.format_metric("aggregate_errors_permanent_total", "", agg.permanent as f64));
        output.push_str(&self.format_metric("aggregate_errors_timeout_total", "", agg.timeout as f64));
        output.push_str(&self.format_metric("aggregate_errors_overload_total", "", agg.overload as f64));
        output.push_str(&self.format_metric("aggregate_errors_degraded_total", "", agg.degraded as f64));
        output.push_str(&self.format_metric("aggregate_errors_partial_failure_total", "", agg.partial_failure as f64));

        output
    }

    fn content_type(&self) -> &'static str {
        "text/plain; version=0.0.4; charset=utf-8"
    }
}

// ─── JSON Exporter ──────────────────────────────────────────────────────────

/// Exports metrics as JSON using proper serialization.
pub struct JsonExporter;

impl JsonExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsExporter for JsonExporter {
    fn export(&self, registry: &MetricsRegistry) -> String {
        let snapshot = registry.snapshot();
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}
