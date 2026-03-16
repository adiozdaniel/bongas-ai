//! ML configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for machine learning model paths, batch sizes,
//! ONNX runtime settings, and Netflix-grade resilience parameters for
//! circuit breaker, bulkhead, retry, fallback, and analytics integration.

use std::path::PathBuf;
use std::time::Duration;

/// ML configuration.
///
/// Configuration for machine learning model paths, batch sizes,
/// ONNX runtime settings, and resilience parameters.
#[derive(Debug, Clone)]
pub struct MlConfig {
    // ── Model Runtime ────────────────────────────────────────────────────────
    pub model_path: PathBuf,
    pub batch_size: usize,
    pub onnx_enabled: bool,
    pub onnx_execution_provider: String,
    pub onnx_graph_optimization: bool,
    pub onnx_memory_map: bool,
    pub onnx_intra_threads: usize,

    // ── Feature Store ────────────────────────────────────────────────────────
    pub feature_store_enabled: bool,
    pub feature_cache_ttl: Duration,
    pub feature_fetch_timeout: Duration,

    // ── Model Registry ───────────────────────────────────────────────────────
    pub model_cache_size: usize,
    pub canary_enabled: bool,
    pub canary_traffic_percent: f64,
    pub shadow_mode_enabled: bool,

    // ── Online Learning ──────────────────────────────────────────────────────
    pub online_learning_enabled: bool,
    pub feedback_batch_size: usize,
    pub feedback_flush_interval: Duration,

    // ── Circuit Breaker (per-model inference) ────────────────────────────────
    pub inference_breaker_failure_rate: f64,
    pub inference_breaker_slow_call_rate: f64,
    pub inference_breaker_slow_call_duration: Duration,
    pub inference_breaker_minimum_calls: u64,
    pub inference_breaker_recovery_timeout: Duration,
    pub inference_breaker_half_open_calls: usize,

    // ── Bulkhead (concurrency limiting) ──────────────────────────────────────
    pub inference_max_concurrent: usize,
    pub feature_fetch_max_concurrent: usize,
    pub worker_queue_depth: usize,

    // ── Retry (resilient loading) ───────────────────────────────────────────
    pub model_load_max_retries: usize,
    pub model_load_base_backoff: Duration,
    pub model_load_max_backoff: Duration,
    pub feature_fetch_max_retries: usize,

    // ── Timeout (latency safety) ────────────────────────────────────────────
    pub inference_timeout: Duration,
    pub model_load_timeout: Duration,

    // ── Fallback (availability priority) ────────────────────────────────────
    pub fallback_to_stale_model: bool,
    pub fallback_cold_start_score: f32,
    pub fallback_max_stale_age: Duration,

    // ── Analytics & Monitoring ──────────────────────────────────────────────
    pub analytics_enabled: bool,
    pub analytics_sample_rate: f64,

    // ── Training Orchestrator (One-Shot Harvest) ─────────────────────────────
    pub central_server_url: String,

    // ── Behavioral Tribes ────────────────────────────────────────────────────
    pub tribe_num_clusters: usize,
    pub tribe_clustering_interval: Duration,

    // ── Content Fatigue ──────────────────────────────────────────────────────
    pub fatigue_enabled: bool,
    pub fatigue_adaptor: ExposureSourceAdaptor,
    pub fatigue_max_exposures: u32,
    pub fatigue_penalty_factor: f32,

    // ── Signal Decay (Cost Management) ───────────────────────────────────────
    pub retention_days: u32,
}

/// Adaptors for tracking item exposure and resetting fatigue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposureSourceAdaptor {
    /// Increments exposures directly during the request/response flow.
    InternalHook,
    /// Consumes exposure and engagement events from Kafka.
    KafkaStream,
    /// Polls ClickHouse for recent exposures and engagements.
    ClickHousePoll,
}

impl Default for MlConfig {
    fn default() -> Self {
        Self {
            // Model Runtime
            model_path: PathBuf::from("./models"),
            batch_size: 64,
            onnx_enabled: true,
            onnx_execution_provider: "cpu".to_string(),
            onnx_graph_optimization: true,
            onnx_memory_map: true,
            onnx_intra_threads: 4,

            // Feature Store
            feature_store_enabled: true,
            feature_cache_ttl: Duration::from_secs(300),
            feature_fetch_timeout: Duration::from_millis(500),

            // Model Registry
            model_cache_size: 100,
            canary_enabled: false,
            canary_traffic_percent: 5.0,
            shadow_mode_enabled: false,

            // Online Learning
            online_learning_enabled: false,
            feedback_batch_size: 512,
            feedback_flush_interval: Duration::from_secs(30),

            // Circuit Breaker
            inference_breaker_failure_rate: 0.5,
            inference_breaker_slow_call_rate: 0.5,
            inference_breaker_slow_call_duration: Duration::from_secs(2),
            inference_breaker_minimum_calls: 10,
            inference_breaker_recovery_timeout: Duration::from_secs(30),
            inference_breaker_half_open_calls: 3,

            // Bulkhead
            inference_max_concurrent: 16,
            feature_fetch_max_concurrent: 32,
            worker_queue_depth: 1024,

            // Retry
            model_load_max_retries: 3,
            model_load_base_backoff: Duration::from_millis(100),
            model_load_max_backoff: Duration::from_secs(5),
            feature_fetch_max_retries: 2,

            // Timeout
            inference_timeout: Duration::from_secs(5),
            model_load_timeout: Duration::from_secs(30),

            // Fallback
            fallback_to_stale_model: true,
            fallback_cold_start_score: 0.5,
            fallback_max_stale_age: Duration::from_secs(3600),

            // Analytics
            analytics_enabled: true,
            analytics_sample_rate: 1.0,
            central_server_url: "https://ml.bongas-ai.com".to_string(),

            // Behavioral Tribes
            tribe_num_clusters: 100,
            tribe_clustering_interval: Duration::from_secs(14400), // 4 hours

            // Content Fatigue
            fatigue_enabled: true,
            fatigue_adaptor: ExposureSourceAdaptor::InternalHook,
            fatigue_max_exposures: 5,
            fatigue_penalty_factor: 0.8,

            // Signal Decay
            retention_days: 90,
        }
    }
}

impl MlConfig {
    /// Create a production-grade ML configuration.
    pub fn production() -> Self {
        Self {
            onnx_intra_threads: 8, // 2x threads
            feature_cache_ttl: Duration::from_secs(600), // Longer cache for production
            inference_max_concurrent: 64, // 4x workers
            feature_fetch_max_concurrent: 128, // 4x fetch concurrency
            worker_queue_depth: 4096, // 4x queue depth
            analytics_sample_rate: 0.1, // Sample 10% in production to reduce overhead
            ..Self::default()
        }
    }
}
