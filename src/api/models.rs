use serde::{Deserialize, Serialize};
use chrono::Utc;

// ========== REQUEST MODELS ==========

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ========== RESPONSE MODELS ==========

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub title: String,
    pub thumbnail_url: String,
    pub score: f32,
    pub rank: i32,
    pub metadata: serde_json::Value,
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
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
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
