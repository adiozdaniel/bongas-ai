//! System and operational models (health, cache, kafka, model management, security).

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct KafkaMetricsResponse {
    pub metrics: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct KafkaHealthResponse {
    pub healthy: bool,
    pub total_consumers: usize,
    pub unhealthy_consumers: Vec<String>,
    pub total_lag: u64,
    pub global_success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelReloadResponse {
    pub model_count: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ModelStatsResponse {
    pub loaded_models: usize,
}

#[derive(Debug, Serialize)]
pub struct SecurityStatusResponse {
    pub validated: bool,
    pub security_enabled: bool,
    pub layers_configured: u32,
}
