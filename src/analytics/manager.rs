use std::sync::Arc;
use std::time::Instant;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec, Encoder, TextEncoder
};
use serde::{Deserialize, Serialize};

/// Central analytics manager for the entire system
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
}

impl AnalyticsManager {
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
        })
    }

    // Request tracking
    pub fn record_recommendation_request(&self, scenario: &str, user_type: &str) {
        self.recommendation_requests
            .with_label_values(&[scenario, user_type])
            .inc();
    }

    pub fn record_recommendation_success(&self, scenario: &str, user_type: &str) {
        self.recommendation_successes
            .with_label_values(&[scenario, user_type])
            .inc();
    }

    pub fn record_recommendation_failure(&self, scenario: &str, error_type: &str) {
        self.recommendation_failures
            .with_label_values(&[scenario, error_type])
            .inc();
    }

    // Latency tracking
    pub fn record_recommendation_latency(&self, scenario: &str, duration: std::time::Duration) {
        self.recommendation_latency
            .with_label_values(&[scenario])
            .observe(duration.as_secs_f64());
    }

    pub fn record_model_inference_latency(&self, model_name: &str, duration: std::time::Duration) {
        self.model_inference_latency
            .with_label_values(&[model_name])
            .observe(duration.as_secs_f64());
    }

    pub fn record_cache_lookup_latency(&self, cache_type: &str, duration: std::time::Duration) {
        self.cache_lookup_latency
            .with_label_values(&[cache_type])
            .observe(duration.as_secs_f64());
    }

    // Cache performance
    pub fn record_cache_hit(&self, cache_type: &str, key_type: &str) {
        self.cache_hits
            .with_label_values(&[cache_type, key_type])
            .inc();
    }

    pub fn record_cache_miss(&self, cache_type: &str, key_type: &str) {
        self.cache_misses
            .with_label_values(&[cache_type, key_type])
            .inc();
    }

    pub fn record_cache_eviction(&self, cache_type: &str) {
        self.cache_evictions
            .with_label_values(&[cache_type])
            .inc();
    }

    // Model performance
    pub fn set_model_accuracy(&self, model_name: &str, accuracy: f64) {
        self.model_accuracy
            .with_label_values(&[model_name])
            .set(accuracy);
    }

    pub fn record_model_prediction(&self, model_name: &str, prediction_type: &str) {
        self.model_predictions
            .with_label_values(&[model_name, prediction_type])
            .inc();
    }

    // Bandit algorithms
    pub fn record_bandit_selection(&self, algorithm: &str, arm: &str) {
        self.bandit_selections
            .with_label_values(&[algorithm, arm])
            .inc();
    }

    pub fn record_bandit_reward(&self, algorithm: &str, reward_type: &str, reward_value: f64) {
        self.bandit_rewards
            .with_label_values(&[algorithm, reward_type])
            .inc_by(reward_value);
    }

    // ClickHouse queries
    pub fn record_clickhouse_query(&self, query_type: &str, table: &str) {
        self.clickhouse_queries
            .with_label_values(&[query_type, table])
            .inc();
    }

    pub fn record_clickhouse_query_latency(&self, query_type: &str, duration: std::time::Duration) {
        self.clickhouse_query_latency
            .with_label_values(&[query_type])
            .observe(duration.as_secs_f64());
    }

    // Convenience methods for timing
    pub fn start_recommendation_timer(&self, scenario: &str) -> RecommendationTimer {
        RecommendationTimer::new(self, scenario)
    }

    pub fn start_model_inference_timer(&self, model_name: &str) -> ModelInferenceTimer {
        ModelInferenceTimer::new(self, model_name)
    }

    pub fn start_cache_lookup_timer(&self, cache_type: &str) -> CacheLookupTimer {
        CacheLookupTimer::new(self, cache_type)
    }

    pub fn start_clickhouse_query_timer(&self, query_type: &str) -> ClickHouseQueryTimer {
        ClickHouseQueryTimer::new(self, query_type)
    }

    // Get metrics as Prometheus format
    pub fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

// Timer helpers for automatic latency recording
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

// Metrics summary for health checks and monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsMetricsSummary {
    pub total_recommendations: u64,
    pub success_rate: f64,
    pub avg_recommendation_latency: f64,
    pub cache_hit_rate: f64,
    pub active_models: Vec<String>,
    pub bandit_algorithms: Vec<String>,
}

impl AnalyticsManager {
    pub async fn get_summary(&self) -> AnalyticsMetricsSummary {
        // This would need to be implemented based on actual metric values
        // For now, returning placeholder values
        AnalyticsMetricsSummary {
            total_recommendations: 0,
            success_rate: 0.0,
            avg_recommendation_latency: 0.0,
            cache_hit_rate: 0.0,
            active_models: vec![],
            bandit_algorithms: vec![],
        }
    }

    /// Track successful scenario loading
    pub async fn track_load_success(&self, scenario_slug: &str, uses_onnx: bool) {
        // Implementation would track scenario loading metrics
    }

    /// Track failed scenario loading
    pub async fn track_load_failure(&self, scenario_slug: &str, error: &str) {
        // Implementation would track scenario loading failures
    }

    /// Track factory summary metrics
    pub async fn track_factory_summary(&self, total_scenarios: usize, onnx_count: usize) {
        // Implementation would track aggregate factory metrics
    }
}

// Global analytics manager instance
lazy_static::lazy_static! {
    pub static ref ANALYTICS_MANAGER: Arc<AnalyticsManager> = {
        Arc::new(AnalyticsManager::new().expect("Failed to create analytics manager"))
    };
}

// Convenience functions for common operations
pub fn record_recommendation_request(scenario: &str, user_type: &str) {
    ANALYTICS_MANAGER.record_recommendation_request(scenario, user_type);
}

pub fn record_recommendation_success(scenario: &str, user_type: &str) {
    ANALYTICS_MANAGER.record_recommendation_success(scenario, user_type);
}

pub fn record_recommendation_failure(scenario: &str, error_type: &str) {
    ANALYTICS_MANAGER.record_recommendation_failure(scenario, error_type);
}

pub fn start_recommendation_timer(scenario: &str) -> RecommendationTimer<'static> {
    ANALYTICS_MANAGER.start_recommendation_timer(scenario)
}

pub fn start_model_inference_timer(model_name: &str) -> ModelInferenceTimer<'static> {
    ANALYTICS_MANAGER.start_model_inference_timer(model_name)
}

pub fn start_cache_lookup_timer(cache_type: &str) -> CacheLookupTimer<'static> {
    ANALYTICS_MANAGER.start_cache_lookup_timer(cache_type)
}

pub fn start_clickhouse_query_timer(query_type: &str) -> ClickHouseQueryTimer<'static> {
    ANALYTICS_MANAGER.start_clickhouse_query_timer(query_type)
}
