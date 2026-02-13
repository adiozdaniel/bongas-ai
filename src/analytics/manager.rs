//! Centralized metrics collection and monitoring system.
//!
//! Provides comprehensive observability for recommendation engine operations including:
//! * Request throughput and success rates
//! * Latency distributions across components
//! * Cache effectiveness and eviction patterns
//! * Model performance and prediction accuracy
//! * Multi-armed bandit algorithm tracking
//! * Database query performance
//! * Scenario lifecycle monitoring
//!
//! All metrics are automatically exported in Prometheus format for integration with
//! standard monitoring stacks (Prometheus, Grafana, Datadog, etc.).

use std::sync::Arc;
use std::time::Instant;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec, Encoder, TextEncoder
};
use serde::{Deserialize, Serialize};

/// Central analytics manager for the entire system.
///
/// Maintains all Prometheus metric vectors and provides typed methods for recording
/// operational telemetry. Designed as a global singleton accessible throughout the
/// application via the `ANALYTICS_MANAGER` lazy static.
pub struct AnalyticsManager {
    /// Request metrics
    recommendation_requests: CounterVec,
    recommendation_successes: CounterVec,
    recommendation_failures: CounterVec,
    
    /// Latency metrics
    recommendation_latency: HistogramVec,
    model_inference_latency: HistogramVec,
    cache_lookup_latency: HistogramVec,
    
    /// Cache performance metrics
    cache_hits: CounterVec,
    cache_misses: CounterVec,
    cache_evictions: CounterVec,
    
    /// Model performance metrics
    model_accuracy: GaugeVec,
    model_predictions: CounterVec,
    
    /// Bandit algorithm metrics
    bandit_selections: CounterVec,
    bandit_rewards: CounterVec,
    
    /// ClickHouse query metrics
    clickhouse_queries: CounterVec,
    clickhouse_query_latency: HistogramVec,
    
    /// Scenario loading metrics
    scenario_loads: CounterVec,
    scenario_load_failures: CounterVec,
    scenarios_loaded: GaugeVec,
}

impl AnalyticsManager {
    /// Initializes all Prometheus metric vectors with appropriate labels and buckets.
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Configured analytics manager or registration error
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            recommendation_requests: register_counter_vec!(
                "bongas_recommendation_requests_total",
                "Total number of recommendation requests",
                &["scenario", "user_type"]
            )?,
            recommendation_successes: register_counter_vec!(
                "bongas_recommendation_successes_total",
                "Total number of successful recommendations",
                &["scenario", "user_type"]
            )?,
            recommendation_failures: register_counter_vec!(
                "bongas_recommendation_failures_total",
                "Total number of failed recommendations",
                &["scenario", "error_type"]
            )?,
            
            recommendation_latency: register_histogram_vec!(
                "bongas_recommendation_latency_seconds",
                "Time taken to generate recommendations",
                &["scenario"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
            )?,
            model_inference_latency: register_histogram_vec!(
                "bongas_model_inference_latency_seconds",
                "Time taken for model inference",
                &["model_name"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
            )?,
            cache_lookup_latency: register_histogram_vec!(
                "bongas_cache_lookup_latency_seconds",
                "Time taken for cache lookups",
                &["cache_type"],
                vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
            )?,
            
            cache_hits: register_counter_vec!(
                "bongas_cache_hits_total",
                "Total number of cache hits",
                &["cache_type", "key_type"]
            )?,
            cache_misses: register_counter_vec!(
                "bongas_cache_misses_total",
                "Total number of cache misses",
                &["cache_type", "key_type"]
            )?,
            cache_evictions: register_counter_vec!(
                "bongas_cache_evictions_total",
                "Total number of cache evictions",
                &["cache_type"]
            )?,
            
            model_accuracy: register_gauge_vec!(
                "bongas_model_accuracy",
                "Current model accuracy",
                &["model_name"]
            )?,
            model_predictions: register_counter_vec!(
                "bongas_model_predictions_total",
                "Total number of model predictions",
                &["model_name", "prediction_type"]
            )?,
            
            bandit_selections: register_counter_vec!(
                "bongas_bandit_selections_total",
                "Total number of bandit algorithm selections",
                &["algorithm", "arm"]
            )?,
            bandit_rewards: register_counter_vec!(
                "bongas_bandit_rewards_total",
                "Total rewards from bandit algorithms",
                &["algorithm", "reward_type"]
            )?,
            
            clickhouse_queries: register_counter_vec!(
                "bongas_clickhouse_queries_total",
                "Total number of ClickHouse queries",
                &["query_type", "table"]
            )?,
            clickhouse_query_latency: register_histogram_vec!(
                "bongas_clickhouse_query_latency_seconds",
                "Time taken for ClickHouse queries",
                &["query_type"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
            )?,
            scenario_loads: register_counter_vec!(
                "bongas_scenario_loads_total",
                "Total number of successful scenario loads",
                &["scenario_slug", "uses_onnx"]
            )?,
            scenario_load_failures: register_counter_vec!(
                "bongas_scenario_load_failures_total",
                "Total number of failed scenario loads",
                &["scenario_slug", "error_type"]
            )?,
            scenarios_loaded: register_gauge_vec!(
                "bongas_scenarios_loaded",
                "Current number of loaded scenarios",
                &["uses_onnx"]
            )?,
        })
    }

    // Request tracking

    /// Increments the counter for recommendation requests.
    ///
    /// # Arguments
    /// * `scenario` - Scenario identifier for segmentation
    /// * `user_type` - User classification (e.g., "anonymous", "authenticated", "premium")
    pub fn record_recommendation_request(&self, scenario: &str, user_type: &str) {
        self.recommendation_requests
            .with_label_values(&[scenario, user_type])
            .inc();
    }

    /// Increments the counter for successful recommendation responses.
    ///
    /// # Arguments
    /// * `scenario` - Scenario identifier for segmentation
    /// * `user_type` - User classification
    pub fn record_recommendation_success(&self, scenario: &str, user_type: &str) {
        self.recommendation_successes
            .with_label_values(&[scenario, user_type])
            .inc();
    }

    /// Increments the counter for failed recommendation requests.
    ///
    /// # Arguments
    /// * `scenario` - Scenario identifier for segmentation
    /// * `error_type` - Classification of failure (e.g., "timeout", "model_unavailable", "invalid_input")
    pub fn record_recommendation_failure(&self, scenario: &str, error_type: &str) {
        self.recommendation_failures
            .with_label_values(&[scenario, error_type])
            .inc();
    }

    // Latency tracking

    /// Records the duration of a recommendation generation operation.
    ///
    /// # Arguments
    /// * `scenario` - Scenario identifier for segmentation
    /// * `duration` - Measured execution time
    pub fn record_recommendation_latency(&self, scenario: &str, duration: std::time::Duration) {
        self.recommendation_latency
            .with_label_values(&[scenario])
            .observe(duration.as_secs_f64());
    }

    /// Records the duration of a model inference operation.
    ///
    /// # Arguments
    /// * `model_name` - Identifier of the model used
    /// * `duration` - Measured inference time
    pub fn record_model_inference_latency(&self, model_name: &str, duration: std::time::Duration) {
        self.model_inference_latency
            .with_label_values(&[model_name])
            .observe(duration.as_secs_f64());
    }

    /// Records the duration of a cache lookup operation.
    ///
    /// # Arguments
    /// * `cache_type` - Cache tier identifier (e.g., "redis", "lru", "in_memory")
    /// * `duration` - Measured lookup time
    pub fn record_cache_lookup_latency(&self, cache_type: &str, duration: std::time::Duration) {
        self.cache_lookup_latency
            .with_label_values(&[cache_type])
            .observe(duration.as_secs_f64());
    }

    // Cache performance

    /// Records a successful cache hit.
    ///
    /// # Arguments
    /// * `cache_type` - Cache tier identifier
    /// * `key_type` - Classification of cached data (e.g., "recommendation", "model_weights", "features")
    pub fn record_cache_hit(&self, cache_type: &str, key_type: &str) {
        self.cache_hits
            .with_label_values(&[cache_type, key_type])
            .inc();
    }

    /// Records a cache miss requiring fallback computation.
    ///
    /// # Arguments
    /// * `cache_type` - Cache tier identifier
    /// * `key_type` - Classification of cached data
    pub fn record_cache_miss(&self, cache_type: &str, key_type: &str) {
        self.cache_misses
            .with_label_values(&[cache_type, key_type])
            .inc();
    }

    /// Records an eviction from a cache.
    ///
    /// # Arguments
    /// * `cache_type` - Cache tier identifier
    pub fn record_cache_eviction(&self, cache_type: &str) {
        self.cache_evictions
            .with_label_values(&[cache_type])
            .inc();
    }

    // Model performance

    /// Sets the current accuracy metric for a model.
    ///
    /// # Arguments
    /// * `model_name` - Identifier of the model
    /// * `accuracy` - Current accuracy score (0.0 - 1.0)
    pub fn set_model_accuracy(&self, model_name: &str, accuracy: f64) {
        self.model_accuracy
            .with_label_values(&[model_name])
            .set(accuracy);
    }

    /// Records a model prediction event.
    ///
    /// # Arguments
    /// * `model_name` - Identifier of the model
    /// * `prediction_type` - Classification of prediction (e.g., "click", "conversion", "rating")
    pub fn record_model_prediction(&self, model_name: &str, prediction_type: &str) {
        self.model_predictions
            .with_label_values(&[model_name, prediction_type])
            .inc();
    }

    // Bandit algorithms

    /// Records a selection by a multi-armed bandit algorithm.
    ///
    /// # Arguments
    /// * `algorithm` - Bandit algorithm identifier (e.g., "epsilon_greedy", "ucb1", "thompson")
    /// * `arm` - Selected arm identifier
    pub fn record_bandit_selection(&self, algorithm: &str, arm: &str) {
        self.bandit_selections
            .with_label_values(&[algorithm, arm])
            .inc();
    }

    /// Records a reward received by a bandit algorithm.
    ///
    /// # Arguments
    /// * `algorithm` - Bandit algorithm identifier
    /// * `reward_type` - Classification of reward (e.g., "click", "purchase", "engagement")
    /// * `reward_value` - Magnitude of the reward
    pub fn record_bandit_reward(&self, algorithm: &str, reward_type: &str, reward_value: f64) {
        self.bandit_rewards
            .with_label_values(&[algorithm, reward_type])
            .inc_by(reward_value);
    }

    // ClickHouse queries

    /// Records execution of a ClickHouse database query.
    ///
    /// # Arguments
    /// * `query_type` - Operation classification (e.g., "select", "insert", "create")
    /// * `table` - Target table name
    pub fn record_clickhouse_query(&self, query_type: &str, table: &str) {
        self.clickhouse_queries
            .with_label_values(&[query_type, table])
            .inc();
    }

    /// Records the duration of a ClickHouse query execution.
    ///
    /// # Arguments
    /// * `query_type` - Operation classification
    /// * `duration` - Measured query execution time
    pub fn record_clickhouse_query_latency(&self, query_type: &str, duration: std::time::Duration) {
        self.clickhouse_query_latency
            .with_label_values(&[query_type])
            .observe(duration.as_secs_f64());
    }

    // Convenience methods for timing

    /// Returns a timer that automatically records recommendation latency on drop.
    ///
    /// # Arguments
    /// * `scenario` - Scenario identifier for segmentation
    ///
    /// # Returns
    /// * `RecommendationTimer` - RAII timer for automatic latency recording
    pub fn start_recommendation_timer(&self, scenario: &str) -> RecommendationTimer<'_> {
        RecommendationTimer::new(self, scenario)
    }

    /// Returns a timer that automatically records model inference latency on drop.
    ///
    /// # Arguments
    /// * `model_name` - Identifier of the model
    ///
    /// # Returns
    /// * `ModelInferenceTimer` - RAII timer for automatic latency recording
    pub fn start_model_inference_timer(&self, model_name: &str) -> ModelInferenceTimer<'_> {
        ModelInferenceTimer::new(self, model_name)
    }

    /// Returns a timer that automatically records cache lookup latency on drop.
    ///
    /// # Arguments
    /// * `cache_type` - Cache tier identifier
    ///
    /// # Returns
    /// * `CacheLookupTimer` - RAII timer for automatic latency recording
    pub fn start_cache_lookup_timer(&self, cache_type: &str) -> CacheLookupTimer<'_> {
        CacheLookupTimer::new(self, cache_type)
    }

    /// Returns a timer that automatically records ClickHouse query latency on drop.
    ///
    /// # Arguments
    /// * `query_type` - Operation classification
    ///
    /// # Returns
    /// * `ClickHouseQueryTimer` - RAII timer for automatic latency recording
    pub fn start_clickhouse_query_timer(&self, query_type: &str) -> ClickHouseQueryTimer<'_> {
        ClickHouseQueryTimer::new(self, query_type)
    }

    /// Gathers all registered metrics and encodes them in Prometheus text format.
    ///
    /// # Returns
    /// * `Result<String, Box<dyn std::error::Error>>` - Prometheus-formatted metrics or encoding error
    pub fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Records a successful scenario load event.
    ///
    /// # Arguments
    /// * `scenario_slug` - Unique scenario identifier
    /// * `uses_onnx` - Whether the scenario uses ONNX runtime for inference
    pub async fn track_load_success(&self, scenario_slug: &str, uses_onnx: bool) {
        self.scenario_loads
            .with_label_values(&[scenario_slug, if uses_onnx { "true" } else { "false" }])
            .inc();
    }

    /// Records a failed scenario load attempt.
    ///
    /// # Arguments
    /// * `scenario_slug` - Unique scenario identifier
    /// * `error` - Error classification for failure analysis
    pub async fn track_load_failure(&self, scenario_slug: &str, error: &str) {
        self.scenario_load_failures
            .with_label_values(&[scenario_slug, error])
            .inc();
    }

    /// Updates the gauge tracking currently loaded scenarios.
    ///
    /// # Arguments
    /// * `total_scenarios` - Total number of loaded scenarios
    /// * `onnx_count` - Number of scenarios using ONNX runtime
    pub async fn track_factory_summary(&self, total_scenarios: usize, onnx_count: usize) {
        self.scenarios_loaded
            .with_label_values(&["false"])
            .set((total_scenarios - onnx_count) as f64);
        self.scenarios_loaded
            .with_label_values(&["true"])
            .set(onnx_count as f64);
    }

    /// Generates a high-level summary of system health and performance.
    ///
    /// Aggregates data from all registered metrics to produce operational KPIs.
    ///
    /// # Returns
    /// * `AnalyticsMetricsSummary` - Structured summary for health checks and dashboards
    pub async fn get_summary(&self) -> AnalyticsMetricsSummary {
        let metric_families = prometheus::gather();
        
        let mut total_recommendations = 0u64;
        let mut successful_recommendations = 0u64;
        let mut total_latency_sum = 0.0;
        let mut latency_count = 0u64;
        let mut cache_hits = 0u64;
        let mut cache_misses = 0u64;
        let mut active_models = std::collections::HashSet::new();
        let mut bandit_algorithms = std::collections::HashSet::new();

        for family in metric_families {
            for metric in family.get_metric() {
                match family.get_name() {
                    "bongas_recommendation_requests_total" => {
                        total_recommendations += metric.get_counter().get_value() as u64;
                    }
                    "bongas_recommendation_successes_total" => {
                        successful_recommendations += metric.get_counter().get_value() as u64;
                    }
                    "bongas_recommendation_latency_seconds_sum" => {
                        total_latency_sum += metric.get_histogram().get_sample_sum();
                    }
                    "bongas_recommendation_latency_seconds_count" => {
                        latency_count += metric.get_histogram().get_sample_count();
                    }
                    "bongas_cache_hits_total" => {
                        cache_hits += metric.get_counter().get_value() as u64;
                    }
                    "bongas_cache_misses_total" => {
                        cache_misses += metric.get_counter().get_value() as u64;
                    }
                    "bongas_model_accuracy" => {
                        if let Some(model_name) = metric.get_label().iter()
                            .find(|label| label.get_name() == "model_name") {
                            active_models.insert(model_name.get_value().to_string());
                        }
                    }
                    "bongas_bandit_selections_total" => {
                        if let Some(algorithm) = metric.get_label().iter()
                            .find(|label| label.get_name() == "algorithm") {
                            bandit_algorithms.insert(algorithm.get_value().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        let success_rate = if total_recommendations > 0 {
            successful_recommendations as f64 / total_recommendations as f64
        } else {
            0.0
        };

        let avg_recommendation_latency = if latency_count > 0 {
            total_latency_sum / latency_count as f64
        } else {
            0.0
        };

        let cache_hit_rate = if cache_hits + cache_misses > 0 {
            cache_hits as f64 / (cache_hits + cache_misses) as f64
        } else {
            0.0
        };

        AnalyticsMetricsSummary {
            total_recommendations,
            success_rate,
            avg_recommendation_latency,
            cache_hit_rate,
            active_models: active_models.into_iter().collect(),
            bandit_algorithms: bandit_algorithms.into_iter().collect(),
        }
    }
}

// Timer helpers for automatic latency recording

/// RAII timer for recommendation operations.
///
/// Records latency to the `recommendation_latency` histogram when dropped.
pub struct RecommendationTimer<'a> {
    manager: &'a AnalyticsManager,
    scenario: String,
    start_time: Instant,
}

impl<'a> RecommendationTimer<'a> {
    fn new(manager: &'a AnalyticsManager, scenario: &str) -> Self {
        Self {
            manager,
            scenario: scenario.to_string(),
            start_time: Instant::now(),
        }
    }
}

impl<'a> Drop for RecommendationTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.manager.record_recommendation_latency(&self.scenario, duration);
    }
}

/// RAII timer for model inference operations.
///
/// Records latency to the `model_inference_latency` histogram when dropped.
pub struct ModelInferenceTimer<'a> {
    manager: &'a AnalyticsManager,
    model_name: String,
    start_time: Instant,
}

impl<'a> ModelInferenceTimer<'a> {
    fn new(manager: &'a AnalyticsManager, model_name: &str) -> Self {
        Self {
            manager,
            model_name: model_name.to_string(),
            start_time: Instant::now(),
        }
    }
}

impl<'a> Drop for ModelInferenceTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.manager.record_model_inference_latency(&self.model_name, duration);
    }
}

/// RAII timer for cache lookup operations.
///
/// Records latency to the `cache_lookup_latency` histogram when dropped.
pub struct CacheLookupTimer<'a> {
    manager: &'a AnalyticsManager,
    cache_type: String,
    start_time: Instant,
}

impl<'a> CacheLookupTimer<'a> {
    fn new(manager: &'a AnalyticsManager, cache_type: &str) -> Self {
        Self {
            manager,
            cache_type: cache_type.to_string(),
            start_time: Instant::now(),
        }
    }
}

impl<'a> Drop for CacheLookupTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.manager.record_cache_lookup_latency(&self.cache_type, duration);
    }
}

/// RAII timer for ClickHouse query operations.
///
/// Records latency to the `clickhouse_query_latency` histogram when dropped.
pub struct ClickHouseQueryTimer<'a> {
    manager: &'a AnalyticsManager,
    query_type: String,
    start_time: Instant,
}

impl<'a> ClickHouseQueryTimer<'a> {
    fn new(manager: &'a AnalyticsManager, query_type: &str) -> Self {
        Self {
            manager,
            query_type: query_type.to_string(),
            start_time: Instant::now(),
        }
    }
}

impl<'a> Drop for ClickHouseQueryTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.manager.record_clickhouse_query_latency(&self.query_type, duration);
    }
}

/// Aggregated metrics snapshot for operational visibility.
///
/// Used by health check endpoints, dashboards, and alerting systems.
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsMetricsSummary {
    /// Total recommendation requests processed since application start
    pub total_recommendations: u64,
    /// Percentage of successful recommendations (0.0 - 1.0)
    pub success_rate: f64,
    /// Average recommendation latency in seconds
    pub avg_recommendation_latency: f64,
    /// Cache hit ratio across all cache tiers (0.0 - 1.0)
    pub cache_hit_rate: f64,
    /// Currently loaded models with active accuracy metrics
    pub active_models: Vec<String>,
    /// Bandit algorithms that have recorded selections
    pub bandit_algorithms: Vec<String>,
}

lazy_static::lazy_static! {
    /// Global singleton instance of the analytics manager.
    ///
    /// Accessible from anywhere in the application for centralized metrics collection.
    /// Panics during initialization if metric registration fails.
    pub static ref ANALYTICS_MANAGER: Arc<AnalyticsManager> = {
        Arc::new(AnalyticsManager::new().expect("Failed to create analytics manager"))
    };
}
