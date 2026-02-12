use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

// ============================================================================
// 1. ScenarioConfig (CORE DIFFERENTIATOR)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScenarioConfig {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,

    // JSONB pipeline definition
    pub pipeline: JsonValue,

    // Caching
    pub cache_ttl_seconds: Option<i32>,
    pub use_l2_cache: bool,
    pub staleness_rules: Option<JsonValue>,

    // Experimentation
    pub experiment_config: Option<JsonValue>,

    // Status
    pub enabled: bool,
    pub priority: i32,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub version: i32,
}

/// Typed representation of the JSONB pipeline column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub stages: Vec<PipelineStageConfig>,
    pub fallback_stages: Option<Vec<PipelineStageConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageConfig {
    pub r#type: String,
    pub params: JsonValue,
}

// ============================================================================
// 2. UserFeatures (Feature Store)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserFeatures {
    pub user_id: i32,
    pub genre_affinity: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub watch_patterns: Option<JsonValue>,
    pub features_updated_at: DateTime<Utc>,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// 3. ItemFeatures (Content Features)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ItemFeatures {
    pub item_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub genres: Option<JsonValue>,
    pub tags: Option<JsonValue>,
    pub duration_seconds: Option<i32>,
    pub tfidf_vector: Option<JsonValue>,
    pub view_count: i32,
    pub like_count: i32,
    pub completion_rate: f32,
    pub trending_score: f32,
    pub published_at: Option<DateTime<Utc>>,
    pub features_updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// 4. RecommendationCacheL2 (Staging Manager - L2 Cache)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationCacheL2 {
    pub id: i64,
    pub cache_key: String,
    pub scenario_slug: String,
    pub user_id: Option<i32>,
    pub context_hash: Option<String>,
    pub recommendations: JsonValue,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_stale: bool,
    pub staleness_reason: Option<String>,
    pub hit_count: i32,
    pub last_hit_at: Option<DateTime<Utc>>,
}

// ============================================================================
// 6. ModelRegistry (ML Model Versioning with ONNX support)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModelRegistry {
    pub id: i32,
    pub model_name: String,
    pub version: String,
    pub model_format: String,
    pub file_path: Option<String>,
    pub onnx_model_path: Option<String>,
    pub s3_location: Option<String>,
    pub architecture_config: Option<JsonValue>,
    pub training_metrics: Option<JsonValue>,
    pub onnx_opset_version: Option<i32>,
    pub onnx_input_shapes: Option<JsonValue>,
    pub onnx_output_names: Option<JsonValue>,
    pub onnx_runtime_provider: String,
    pub status: String,
    pub deployed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

