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
    pub l1_items: u64,
    pub l2_items: u64,
    pub evictions: u64,
}

#[derive(Debug, Serialize)]
pub struct KafkaMetricsResponse {
    pub metrics: serde_json::Value,
    pub messages_sent: u64,
    pub messages_failed: u64,
    pub latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct KafkaHealthResponse {
    pub healthy: bool,
    pub total_consumers: usize,
    pub unhealthy_consumers: Vec<String>,
    pub total_lag: u64,
    pub global_success_rate: f64,
    pub status: String,
    pub brokers_online: usize,
}

#[derive(Debug, Serialize)]
pub struct ModelReloadResponse {
    pub model_count: usize,
    pub message: String,
    pub models_reloaded: usize,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ModelStatsResponse {
    pub loaded_models: usize,
    pub active_models: usize,
    pub inference_count: u64,
    pub average_latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct SecurityStatusResponse {
    pub validated: bool,
    pub security_enabled: bool,
    pub layers_configured: u32,
}
