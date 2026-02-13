//! ML model-specific instrumentation.
//!
//! Provides comprehensive Prometheus metrics for monitoring machine learning model
//! lifecycle and performance. Tracks model loading, inference, caching, feature
//! extraction, and resource utilization. Enables observability of model serving
//! infrastructure and prediction quality.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for ML model operations and performance.
///
/// Maintains comprehensive metric vectors for:
/// * Model loading throughput and reliability
/// * Inference request volume and latency
/// * Prediction accuracy and confidence distributions
/// * Model cache effectiveness
/// * Model versioning and deployment lifecycle
/// * Feature engineering pipeline performance
/// * Runtime resource consumption
pub struct ModelMetrics {
    /// Total model load attempts, labeled by model name.
    pub model_loads: IntCounterVec,
    /// Successful model loads, labeled by model name.
    pub model_load_successes: IntCounterVec,
    /// Failed model loads, labeled by model name.
    pub model_load_failures: IntCounterVec,
    /// Model loading latency distribution, labeled by model name.
    pub model_load_latency: HistogramVec,

    /// Total model inference requests, labeled by model name.
    pub model_inferences: IntCounterVec,
    /// Successful model inference operations, labeled by model name.
    pub model_inference_successes: IntCounterVec,
    /// Failed model inference operations, labeled by model name.
    pub model_inference_failures: IntCounterVec,
    /// Model inference latency distribution, labeled by model name.
    pub model_inference_latency: HistogramVec,

    /// Total model predictions generated, labeled by model name.
    pub model_predictions: IntCounterVec,
    /// Prediction accuracy distribution, labeled by model name.
    pub model_accuracy: HistogramVec,
    /// Prediction confidence score distribution, labeled by model name.
    pub model_confidence: HistogramVec,

    /// Model cache hits, labeled by model name.
    pub model_cache_hits: IntCounterVec,
    /// Model cache misses, labeled by model name.
    pub model_cache_misses: IntCounterVec,
    /// Model cache eviction events, labeled by model name.
    pub model_cache_evictions: IntCounterVec,
    /// Model cache lookup latency, labeled by model name.
    pub model_cache_latency: HistogramVec,

    /// Model deployment events, labeled by model name.
    pub model_deployments: IntCounterVec,
    /// Model undeployment events, labeled by model name.
    pub model_undeployments: IntCounterVec,
    /// Model version count, labeled by model name.
    pub model_versions: IntCounterVec,

    /// Total feature extraction attempts, labeled by feature type.
    pub feature_extraction_attempts: IntCounterVec,
    /// Successful feature extraction operations, labeled by feature type.
    pub feature_extraction_successes: IntCounterVec,
    /// Failed feature extraction operations, labeled by feature type.
    pub feature_extraction_failures: IntCounterVec,
    /// Feature extraction latency distribution, labeled by feature type.
    pub feature_extraction_latency: HistogramVec,

    /// Model inference throughput (items/second), labeled by model name.
    pub model_throughput: HistogramVec,
    /// Model memory consumption in bytes, labeled by model name.
    pub model_memory_usage: HistogramVec,
    /// Model CPU utilization percentage, labeled by model name.
    pub model_cpu_usage: HistogramVec,
}

impl ModelMetrics {
    /// Creates and registers all ML model metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Model loading metrics
            model_loads: register_int_counter_vec_with_registry!(
                opts!("model_loads_total", "Total model loads"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_load_successes: register_int_counter_vec_with_registry!(
                opts!("model_load_successes_total", "Total model load successes"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_load_failures: register_int_counter_vec_with_registry!(
                opts!("model_load_failures_total", "Total model load failures"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_load_latency: register_histogram_vec_with_registry!(
                format!("{}_model_load_duration_seconds", NAMESPACE),
                "Model loading latency",
                &[labels::MODEL_NAME],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Model inference metrics
            model_inferences: register_int_counter_vec_with_registry!(
                opts!("model_inferences_total", "Total model inferences"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_inference_successes: register_int_counter_vec_with_registry!(
                opts!("model_inference_successes_total", "Total model inference successes"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_inference_failures: register_int_counter_vec_with_registry!(
                opts!("model_inference_failures_total", "Total model inference failures"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_inference_latency: register_histogram_vec_with_registry!(
                format!("{}_model_inference_duration_seconds", NAMESPACE),
                "Model inference latency",
                &[labels::MODEL_NAME],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Model accuracy metrics
            model_predictions: register_int_counter_vec_with_registry!(
                opts!("model_predictions_total", "Total model predictions"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_accuracy: register_histogram_vec_with_registry!(
                format!("{}_model_accuracy", NAMESPACE),
                "Model prediction accuracy",
                &[labels::MODEL_NAME],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,
            model_confidence: register_histogram_vec_with_registry!(
                format!("{}_model_confidence", NAMESPACE),
                "Model prediction confidence",
                &[labels::MODEL_NAME],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,

            // Model cache metrics
            model_cache_hits: register_int_counter_vec_with_registry!(
                opts!("model_cache_hits_total", "Total model cache hits"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_cache_misses: register_int_counter_vec_with_registry!(
                opts!("model_cache_misses_total", "Total model cache misses"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_cache_evictions: register_int_counter_vec_with_registry!(
                opts!("model_cache_evictions_total", "Total model cache evictions"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_cache_latency: register_histogram_vec_with_registry!(
                format!("{}_model_cache_duration_seconds", NAMESPACE),
                "Model cache lookup latency",
                &[labels::MODEL_NAME],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Model lifecycle metrics
            model_deployments: register_int_counter_vec_with_registry!(
                opts!("model_deployments_total", "Total model deployments"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_undeployments: register_int_counter_vec_with_registry!(
                opts!("model_undeployments_total", "Total model undeployments"),
                &[labels::MODEL_NAME],
                registry
            )?,
            model_versions: register_int_counter_vec_with_registry!(
                opts!("model_versions_total", "Total model versions"),
                &[labels::MODEL_NAME],
                registry
            )?,

            // Feature extraction metrics
            feature_extraction_attempts: register_int_counter_vec_with_registry!(
                opts!("feature_extraction_attempts_total", "Total feature extraction attempts"),
                &[labels::FEATURE_TYPE],
                registry
            )?,
            feature_extraction_successes: register_int_counter_vec_with_registry!(
                opts!("feature_extraction_successes_total", "Total feature extraction successes"),
                &[labels::FEATURE_TYPE],
                registry
            )?,
            feature_extraction_failures: register_int_counter_vec_with_registry!(
                opts!("feature_extraction_failures_total", "Total feature extraction failures"),
                &[labels::FEATURE_TYPE],
                registry
            )?,
            feature_extraction_latency: register_histogram_vec_with_registry!(
                format!("{}_feature_extraction_duration_seconds", NAMESPACE),
                "Feature extraction latency",
                &[labels::FEATURE_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Model performance metrics
            model_throughput: register_histogram_vec_with_registry!(
                format!("{}_model_throughput", NAMESPACE),
                "Model inference throughput (items/second)",
                &[labels::MODEL_NAME],
                vec![0.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0],
                registry
            )?,
            model_memory_usage: register_histogram_vec_with_registry!(
                format!("{}_model_memory_usage_bytes", NAMESPACE),
                "Model memory usage in bytes",
                &[labels::MODEL_NAME],
                vec![0.0, 1000000.0, 10000000.0, 100000000.0, 1000000000.0, 10000000000.0],
                registry
            )?,
            model_cpu_usage: register_histogram_vec_with_registry!(
                format!("{}_model_cpu_usage_percent", NAMESPACE),
                "Model CPU usage percentage",
                &[labels::MODEL_NAME],
                vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0],
                registry
            )?,
        })
    }
}
