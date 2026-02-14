//! Metrics exporters for various output formats.
//!
//! Implements the Strategy pattern — different exporters can be plugged
//! in to output metrics in different formats (Prometheus, JSON, etc.)
//!
//! # Netflix Resilience Features
//! - **Error Classification Export**: Breakdown by classification type
//! - **Degraded/Slow Call Metrics**: Exported separately for alerting

use super::registry::MetricsRegistry;

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
}

impl MetricsExporter for PrometheusExporter {
    fn export(&self, registry: &MetricsRegistry) -> String {
        let snapshot = registry.snapshot();
        let mut output = String::with_capacity(8192);

        // Per-breaker metrics
        for breaker in &snapshot.breakers {
            let labels = format!("breaker=\"{}\"", breaker.breaker_id);

            // Basic counters
            output.push_str(&self.format_metric(
                "calls_total",
                &labels,
                breaker.total_calls as f64,
            ));
            output.push_str(&self.format_metric(
                "successes_total",
                &labels,
                breaker.successes as f64,
            ));
            output.push_str(&self.format_metric(
                "failures_total",
                &labels,
                breaker.failures as f64,
            ));
            output.push_str(&self.format_metric(
                "timeouts_total",
                &labels,
                breaker.timeouts as f64,
            ));
            output.push_str(&self.format_metric(
                "rejections_total",
                &labels,
                breaker.rejections as f64,
            ));
            output.push_str(&self.format_metric(
                "slow_calls_total",
                &labels,
                breaker.slow_calls as f64,
            ));
            output.push_str(&self.format_metric(
                "degraded_calls_total",
                &labels,
                breaker.degraded_calls as f64,
            ));

            // Rates
            output.push_str(&self.format_metric("failure_rate", &labels, breaker.failure_rate));
            output.push_str(&self.format_metric("throughput", &labels, breaker.throughput));

            // Latency percentiles
            output.push_str(&self.format_metric(
                "latency_p50_ms",
                &labels,
                breaker.latency_p50_ms,
            ));
            output.push_str(&self.format_metric(
                "latency_p90_ms",
                &labels,
                breaker.latency_p90_ms,
            ));
            output.push_str(&self.format_metric(
                "latency_p95_ms",
                &labels,
                breaker.latency_p95_ms,
            ));
            output.push_str(&self.format_metric(
                "latency_p99_ms",
                &labels,
                breaker.latency_p99_ms,
            ));
            output.push_str(&self.format_metric(
                "latency_mean_ms",
                &labels,
                breaker.latency_mean_ms,
            ));

            // Gauges
            output.push_str(&self.format_metric(
                "concurrent_calls",
                &labels,
                breaker.concurrent_calls as f64,
            ));
            output.push_str(&self.format_metric(
                "state_duration_seconds",
                &labels,
                breaker.state_duration_secs,
            ));

            // State as a labeled metric
            let state_labels = format!("{},state=\"{}\"", labels, breaker.state);
            output.push_str(&self.format_metric("state", &state_labels, 1.0));

            // Error classification breakdown
            let class = &breaker.classifications;
            output.push_str(&self.format_metric(
                "errors_transient_total",
                &labels,
                class.transient as f64,
            ));
            output.push_str(&self.format_metric(
                "errors_permanent_total",
                &labels,
                class.permanent as f64,
            ));
            output.push_str(&self.format_metric(
                "errors_timeout_total",
                &labels,
                class.timeout as f64,
            ));
            output.push_str(&self.format_metric(
                "errors_overload_total",
                &labels,
                class.overload as f64,
            ));
            output.push_str(&self.format_metric(
                "errors_degraded_total",
                &labels,
                class.degraded as f64,
            ));
            output.push_str(&self.format_metric(
                "errors_partial_failure_total",
                &labels,
                class.partial_failure as f64,
            ));
        }

        // Aggregate metrics
        output.push_str(&self.format_metric(
            "aggregate_calls_total",
            "",
            snapshot.total_calls as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_successes_total",
            "",
            snapshot.total_successes as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_failures_total",
            "",
            snapshot.total_failures as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_timeouts_total",
            "",
            snapshot.total_timeouts as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_rejections_total",
            "",
            snapshot.total_rejections as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_slow_calls_total",
            "",
            snapshot.total_slow_calls as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_degraded_total",
            "",
            snapshot.total_degraded as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_failure_rate",
            "",
            snapshot.aggregate_failure_rate,
        ));
        output.push_str(&self.format_metric(
            "aggregate_throughput",
            "",
            snapshot.aggregate_throughput,
        ));

        // Aggregate classification breakdown
        let agg = &snapshot.aggregate_classifications;
        output.push_str(&self.format_metric(
            "aggregate_errors_transient_total",
            "",
            agg.transient as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_errors_permanent_total",
            "",
            agg.permanent as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_errors_timeout_total",
            "",
            agg.timeout as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_errors_overload_total",
            "",
            agg.overload as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_errors_degraded_total",
            "",
            agg.degraded as f64,
        ));
        output.push_str(&self.format_metric(
            "aggregate_errors_partial_failure_total",
            "",
            agg.partial_failure as f64,
        ));

        output
    }

    fn content_type(&self) -> &'static str {
        "text/plain; version=0.0.4; charset=utf-8"
    }
}

// ─── JSON Exporter ──────────────────────────────────────────────────────────

/// Exports metrics as JSON.
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

        // Manual JSON serialization to avoid serde dependency
        let mut output = String::with_capacity(8192);
        output.push_str("{\n");

        // Breakers array
        output.push_str("  \"breakers\": [\n");
        for (i, breaker) in snapshot.breakers.iter().enumerate() {
            output.push_str("    {\n");
            output.push_str(&format!(
                "      \"breaker_id\": \"{}\",\n",
                breaker.breaker_id
            ));
            output.push_str(&format!("      \"state\": \"{}\",\n", breaker.state));
            output.push_str(&format!("      \"total_calls\": {},\n", breaker.total_calls));
            output.push_str(&format!("      \"successes\": {},\n", breaker.successes));
            output.push_str(&format!("      \"failures\": {},\n", breaker.failures));
            output.push_str(&format!("      \"timeouts\": {},\n", breaker.timeouts));
            output.push_str(&format!("      \"rejections\": {},\n", breaker.rejections));
            output.push_str(&format!("      \"slow_calls\": {},\n", breaker.slow_calls));
            output.push_str(&format!(
                "      \"degraded_calls\": {},\n",
                breaker.degraded_calls
            ));
            output.push_str(&format!(
                "      \"failure_rate\": {:.4},\n",
                breaker.failure_rate
            ));
            output.push_str(&format!("      \"throughput\": {:.2},\n", breaker.throughput));
            output.push_str(&format!(
                "      \"latency_p50_ms\": {:.3},\n",
                breaker.latency_p50_ms
            ));
            output.push_str(&format!(
                "      \"latency_p90_ms\": {:.3},\n",
                breaker.latency_p90_ms
            ));
            output.push_str(&format!(
                "      \"latency_p95_ms\": {:.3},\n",
                breaker.latency_p95_ms
            ));
            output.push_str(&format!(
                "      \"latency_p99_ms\": {:.3},\n",
                breaker.latency_p99_ms
            ));
            output.push_str(&format!(
                "      \"latency_mean_ms\": {:.3},\n",
                breaker.latency_mean_ms
            ));
            output.push_str(&format!(
                "      \"concurrent_calls\": {},\n",
                breaker.concurrent_calls
            ));
            output.push_str(&format!(
                "      \"state_duration_secs\": {:.2},\n",
                breaker.state_duration_secs
            ));

            // Classification breakdown
            let class = &breaker.classifications;
            output.push_str("      \"classifications\": {\n");
            output.push_str(&format!("        \"transient\": {},\n", class.transient));
            output.push_str(&format!("        \"permanent\": {},\n", class.permanent));
            output.push_str(&format!("        \"timeout\": {},\n", class.timeout));
            output.push_str(&format!("        \"overload\": {},\n", class.overload));
            output.push_str(&format!("        \"degraded\": {},\n", class.degraded));
            output.push_str(&format!(
                "        \"partial_failure\": {}\n",
                class.partial_failure
            ));
            output.push_str("      }\n");

            output.push_str("    }");
            if i < snapshot.breakers.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("  ],\n");

        // Aggregates
        output.push_str(&format!("  \"total_calls\": {},\n", snapshot.total_calls));
        output.push_str(&format!(
            "  \"total_successes\": {},\n",
            snapshot.total_successes
        ));
        output.push_str(&format!(
            "  \"total_failures\": {},\n",
            snapshot.total_failures
        ));
        output.push_str(&format!(
            "  \"total_timeouts\": {},\n",
            snapshot.total_timeouts
        ));
        output.push_str(&format!(
            "  \"total_rejections\": {},\n",
            snapshot.total_rejections
        ));
        output.push_str(&format!(
            "  \"total_slow_calls\": {},\n",
            snapshot.total_slow_calls
        ));
        output.push_str(&format!(
            "  \"total_degraded\": {},\n",
            snapshot.total_degraded
        ));
        output.push_str(&format!(
            "  \"aggregate_failure_rate\": {:.4},\n",
            snapshot.aggregate_failure_rate
        ));
        output.push_str(&format!(
            "  \"aggregate_throughput\": {:.2},\n",
            snapshot.aggregate_throughput
        ));

        // Aggregate classifications
        let agg = &snapshot.aggregate_classifications;
        output.push_str("  \"aggregate_classifications\": {\n");
        output.push_str(&format!("    \"transient\": {},\n", agg.transient));
        output.push_str(&format!("    \"permanent\": {},\n", agg.permanent));
        output.push_str(&format!("    \"timeout\": {},\n", agg.timeout));
        output.push_str(&format!("    \"overload\": {},\n", agg.overload));
        output.push_str(&format!("    \"degraded\": {},\n", agg.degraded));
        output.push_str(&format!("    \"partial_failure\": {}\n", agg.partial_failure));
        output.push_str("  }\n");

        output.push_str("}\n");
        output
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}
