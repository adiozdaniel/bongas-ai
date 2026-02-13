//! Pipeline stage-specific instrumentation.
//!
//! Provides Prometheus metrics for monitoring data processing pipeline stages,
//! including stage execution performance, data flow characteristics, ONNX model
//! inference, and end-to-end pipeline completion. Enables observability of
//! batch and streaming data processing workflows.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for data pipeline operations and stage execution.
///
/// Maintains comprehensive metric vectors for:
/// * Pipeline stage throughput and reliability
/// * Stage execution latency distributions
/// * Data volume and filtering efficiency
/// * ONNX model inference performance
/// * Model prediction caching effectiveness
/// * End-to-end pipeline completion rates
pub struct PipelineMetrics {
    /// Total stage executions, labeled by stage name.
    pub stage_executions: IntCounterVec,
    /// Successful stage completions, labeled by stage name.
    pub stage_successes: IntCounterVec,
    /// Failed stage executions, labeled by stage name.
    pub stage_failures: IntCounterVec,
    /// Stage execution latency distribution, labeled by stage name.
    pub stage_execution_latency: HistogramVec,

    /// Number of items entering a stage, labeled by stage name.
    pub stage_input_items: HistogramVec,
    /// Number of items exiting a stage, labeled by stage name.
    pub stage_output_items: HistogramVec,
    /// Filter rate (output/input ratio), labeled by stage name.
    pub stage_filter_rate: HistogramVec,

    /// Complete pipeline executions, labeled by scenario.
    pub pipeline_completions: IntCounterVec,
    /// Pipeline execution failures, labeled by scenario.
    pub pipeline_failures: IntCounterVec,
    /// End-to-end pipeline latency distribution, labeled by scenario.
    pub pipeline_execution_latency: HistogramVec,
}

impl PipelineMetrics {
    /// Creates and registers all pipeline metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Stage execution metrics
            stage_executions: register_int_counter_vec_with_registry!(
                opts!("pipeline_stage_executions_total", "Total pipeline stage executions"),
                &[labels::STAGE],
                registry
            )?,
            stage_successes: register_int_counter_vec_with_registry!(
                opts!("pipeline_stage_successes_total", "Total pipeline stage successes"),
                &[labels::STAGE],
                registry
            )?,
            stage_failures: register_int_counter_vec_with_registry!(
                opts!("pipeline_stage_failures_total", "Total pipeline stage failures"),
                &[labels::STAGE],
                registry
            )?,
            stage_execution_latency: register_histogram_vec_with_registry!(
                format!("{}_pipeline_stage_duration_seconds", NAMESPACE),
                "Pipeline stage execution latency",
                &[labels::STAGE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Stage-specific metrics
            stage_input_items: register_histogram_vec_with_registry!(
                format!("{}_pipeline_stage_input_items", NAMESPACE),
                "Number of items entering a stage",
                &[labels::STAGE],
                vec![0.0, 1.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0],
                registry
            )?,
            stage_output_items: register_histogram_vec_with_registry!(
                format!("{}_pipeline_stage_output_items", NAMESPACE),
                "Number of items exiting a stage",
                &[labels::STAGE],
                vec![0.0, 1.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0],
                registry
            )?,
            stage_filter_rate: register_histogram_vec_with_registry!(
                format!("{}_pipeline_stage_filter_rate", NAMESPACE),
                "Filter rate of a stage (output/input)",
                &[labels::STAGE],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,

            // Pipeline flow metrics
            pipeline_completions: register_int_counter_vec_with_registry!(
                opts!("pipeline_completions_total", "Total pipeline completions"),
                &[labels::SCENARIO],
                registry
            )?,
            pipeline_failures: register_int_counter_vec_with_registry!(
                opts!("pipeline_failures_total", "Total pipeline failures"),
                &[labels::SCENARIO],
                registry
            )?,
            pipeline_execution_latency: register_histogram_vec_with_registry!(
                format!("{}_pipeline_duration_seconds", NAMESPACE),
                "Complete pipeline execution latency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
