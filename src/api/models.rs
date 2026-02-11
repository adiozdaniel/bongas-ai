use serde::{Deserialize, Serialize};
use chrono::Utc;

// ========== REQUEST MODELS ==========

#[derive(Debug, Deserialize)]
pub struct CreateScenarioRequest {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub pipeline: serde_json::Value,  // JSONB pipeline definition
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScenarioRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub pipeline: Option<serde_json::Value>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct InvalidateCacheRequest {
    pub scenario_slug: Option<String>,
    pub profile_id: Option<i32>,
}

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

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
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
pub struct ScenarioResponse {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub pipeline: serde_json::Value,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
pub struct ScenarioExecutionStatsResponse {
    pub scenario_slug: String,
    pub uses_onnx_inference: bool,
    pub pipeline_stage_count: usize,
    pub onnx_stage_count: usize,
    pub execution_time_ms: u64,
    pub cached_result: bool,
}