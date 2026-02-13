//! Scenario factory and configuration-specific instrumentation.
//!
//! Provides comprehensive Prometheus metrics for monitoring scenario lifecycle management,
//! including configuration loading, execution performance, ONNX model deployment,
//! factory operations, and cache invalidation patterns. Enables observability of
//! the scenario management subsystem.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for scenario factory and execution operations.
///
/// Maintains comprehensive metric vectors for:
/// * Scenario configuration loading throughput and reliability
/// * Scenario execution volume and latency distributions
/// * Configuration validation success and error rates
/// * ONNX model deployment lifecycle and performance
/// * Factory operation throughput and response times
/// * Scenario cache effectiveness and invalidation patterns
pub struct ScenarioMetrics {
    /// Total scenario load attempts, labeled by scenario.
    pub scenario_loads: IntCounterVec,
    /// Successful scenario loads, labeled by scenario.
    pub scenario_load_successes: IntCounterVec,
    /// Failed scenario loads, labeled by scenario.
    pub scenario_load_failures: IntCounterVec,
    /// Scenario loading latency distribution, labeled by scenario.
    pub scenario_load_latency: HistogramVec,

    /// Total scenario executions, labeled by scenario.
    pub scenario_executions: IntCounterVec,
    /// Successful scenario executions, labeled by scenario.
    pub scenario_successes: IntCounterVec,
    /// Failed scenario executions, labeled by scenario.
    pub scenario_failures: IntCounterVec,
    /// Scenario execution latency distribution, labeled by scenario.
    pub scenario_execution_latency: HistogramVec,

    /// Total scenario configurations loaded, labeled by scenario.
    pub scenario_configs: IntCounterVec,
    /// Total configuration validation attempts, labeled by scenario.
    pub scenario_config_validations: IntCounterVec,
    /// Total configuration validation errors, labeled by scenario.
    pub scenario_config_errors: IntCounterVec,

    /// Total ONNX model deployment attempts, labeled by model name.
    pub onnx_deployments: IntCounterVec,
    /// Successful ONNX model deployments, labeled by model name.
    pub onnx_deployment_successes: IntCounterVec,
    /// Failed ONNX model deployments, labeled by model name.
    pub onnx_deployment_failures: IntCounterVec,
    /// ONNX model deployment latency distribution, labeled by model name.
    pub onnx_deployment_latency: HistogramVec,

    /// Total scenario factory operations, labeled by operation type.
    pub factory_operations: IntCounterVec,
    /// Successful factory operations, labeled by operation type.
    pub factory_successes: IntCounterVec,
    /// Failed factory operations, labeled by operation type.
    pub factory_failures: IntCounterVec,
    /// Factory operation latency distribution, labeled by operation type.
    pub factory_latency: HistogramVec,

    /// Total scenario staleness checks, labeled by scenario.
    pub scenario_staleness_checks: IntCounterVec,
    /// Total scenario invalidations, labeled by scenario and invalidation reason.
    pub scenario_invalidations: IntCounterVec,
    /// Scenario cache hits, labeled by scenario.
    pub scenario_cache_hits: IntCounterVec,
    /// Scenario cache misses, labeled by scenario.
    pub scenario_cache_misses: IntCounterVec,
}

impl ScenarioMetrics {
    /// Creates and registers all scenario metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Scenario loading metrics
            scenario_loads: register_int_counter_vec_with_registry!(
                opts!("scenario_loads_total", "Total scenario loads"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_load_successes: register_int_counter_vec_with_registry!(
                opts!("scenario_load_successes_total", "Total scenario load successes"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_load_failures: register_int_counter_vec_with_registry!(
                opts!("scenario_load_failures_total", "Total scenario load failures"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_load_latency: register_histogram_vec_with_registry!(
                format!("{}_scenario_load_duration_seconds", NAMESPACE),
                "Scenario loading latency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Scenario execution metrics
            scenario_executions: register_int_counter_vec_with_registry!(
                opts!("scenario_executions_total", "Total scenario executions"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_successes: register_int_counter_vec_with_registry!(
                opts!("scenario_successes_total", "Total scenario execution successes"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_failures: register_int_counter_vec_with_registry!(
                opts!("scenario_failures_total", "Total scenario execution failures"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_execution_latency: register_histogram_vec_with_registry!(
                format!("{}_scenario_execution_duration_seconds", NAMESPACE),
                "Scenario execution latency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Scenario configuration metrics
            scenario_configs: register_int_counter_vec_with_registry!(
                opts!("scenario_configs_total", "Total scenario configurations"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_config_validations: register_int_counter_vec_with_registry!(
                opts!("scenario_config_validations_total", "Total scenario configuration validations"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_config_errors: register_int_counter_vec_with_registry!(
                opts!("scenario_config_errors_total", "Total scenario configuration errors"),
                &[labels::SCENARIO],
                registry
            )?,

            // ONNX model deployment metrics
            onnx_deployments: register_int_counter_vec_with_registry!(
                opts!("onnx_deployments_total", "Total ONNX model deployments"),
                &[labels::MODEL_NAME],
                registry
            )?,
            onnx_deployment_successes: register_int_counter_vec_with_registry!(
                opts!("onnx_deployment_successes_total", "Total ONNX model deployment successes"),
                &[labels::MODEL_NAME],
                registry
            )?,
            onnx_deployment_failures: register_int_counter_vec_with_registry!(
                opts!("onnx_deployment_failures_total", "Total ONNX model deployment failures"),
                &[labels::MODEL_NAME],
                registry
            )?,
            onnx_deployment_latency: register_histogram_vec_with_registry!(
                format!("{}_onnx_deployment_duration_seconds", NAMESPACE),
                "ONNX model deployment latency",
                &[labels::MODEL_NAME],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Scenario factory metrics
            factory_operations: register_int_counter_vec_with_registry!(
                opts!("scenario_factory_operations_total", "Total scenario factory operations"),
                &[labels::SCENARIO],
                registry
            )?,
            factory_successes: register_int_counter_vec_with_registry!(
                opts!("scenario_factory_successes_total", "Total scenario factory operation successes"),
                &[labels::SCENARIO],
                registry
            )?,
            factory_failures: register_int_counter_vec_with_registry!(
                opts!("scenario_factory_failures_total", "Total scenario factory operation failures"),
                &[labels::SCENARIO],
                registry
            )?,
            factory_latency: register_histogram_vec_with_registry!(
                format!("{}_scenario_factory_duration_seconds", NAMESPACE),
                "Scenario factory operation latency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Scenario staleness metrics
            scenario_staleness_checks: register_int_counter_vec_with_registry!(
                opts!("scenario_staleness_checks_total", "Total scenario staleness checks"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_invalidations: register_int_counter_vec_with_registry!(
                opts!("scenario_invalidations_total", "Total scenario invalidations"),
                &[labels::SCENARIO, labels::INVALIDATION_REASON],
                registry
            )?,
            scenario_cache_hits: register_int_counter_vec_with_registry!(
                opts!("scenario_cache_hits_total", "Total scenario cache hits"),
                &[labels::SCENARIO],
                registry
            )?,
            scenario_cache_misses: register_int_counter_vec_with_registry!(
                opts!("scenario_cache_misses_total", "Total scenario cache misses"),
                &[labels::SCENARIO],
                registry
            )?,
        })
    }
}
