use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

// ============================================================================
// 1. Scenario (The Intelligent Brain Core)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, sqlx::Type)]
#[sqlx(transparent)]
pub struct ScenarioId(i32);

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Scenario {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub target_kpi: String,

    // Safety & Targeting Defaults (KFCB Standard)
    pub maturity_rating: String,

    // Configuration
    pub initial_display_limit: i32,
    pub scope: JsonValue,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,

    // Metadata
    pub created_at: DateTime<Utc>,
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
// 2. ProfileFeatures (Feature Store)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileFeatures {
    pub profile_id: String,
    pub user_id: i32,
    pub genre_affinity: Option<JsonValue>,
    pub disliked_genres: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub favorite_creators: Option<JsonValue>,
    pub watch_patterns: Option<JsonValue>,
    pub preferred_content_type: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub features_updated_at: DateTime<Utc>,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VisitorFeatures {
    pub visitor_id: String,
    pub genre_affinity: Option<JsonValue>,
    pub disliked_genres: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub favorite_creators: Option<JsonValue>,
    pub watch_patterns: Option<JsonValue>,
    pub preferred_content_type: Option<String>,
    pub embedding: Option<Vec<f32>>,
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
    // Content metadata
    pub title: Option<String>,
    pub description: Option<String>,
    pub genres: Option<JsonValue>,
    pub tags: Option<JsonValue>,
    pub creators: Option<JsonValue>,
    pub directors: Option<JsonValue>,
    pub studios: Option<JsonValue>,
    pub actors: Option<JsonValue>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub audio_languages: Option<JsonValue>,
    pub subtitle_languages: Option<JsonValue>,
    pub age_rating: Option<String>,
    pub duration_seconds: Option<i32>,
    pub release_year: Option<i32>,
    pub release_date: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub added_date: Option<DateTime<Utc>>,
    pub available_from: Option<DateTime<Utc>>,
    pub available_until: Option<DateTime<Utc>>,
    pub is_active: bool,
    // Quality & Technical
    pub max_resolution: Option<String>,
    pub has_hdr: Option<bool>,
    pub has_dolby_vision: Option<bool>,
    pub has_dolby_atmos: Option<bool>,
    // Content Warnings
    pub is_explicit: Option<bool>,
    pub has_violence: Option<bool>,
    pub has_strong_language: Option<bool>,
    pub has_drug_content: Option<bool>,
    // Country availability
    pub available_countries: Option<JsonValue>,
    pub blocked_countries: Option<JsonValue>,
    // Specialized Tags
    pub seasonal_tags: Option<JsonValue>,
    pub holiday_tags: Option<JsonValue>,
    pub themes: Option<JsonValue>,
    pub is_award_winner: Option<bool>,
    pub required_tier: Option<String>,
    pub is_free: Option<bool>,
    // Scores
    pub view_count: i32,
    pub like_count: i32,
    pub comment_count: Option<i64>,
    pub share_count: Option<i64>,
    pub save_count: Option<i64>,
    pub completion_rate: f32,
    pub trending_score: f32,
    pub popularity_score: Option<f32>,
    pub user_rating: Option<f32>,
    pub user_rating_count: Option<i32>,
    pub critic_rating: Option<f32>,
    pub critic_rating_count: Option<i32>,
    // Embeddings & vectors
    pub embedding: Option<Vec<f32>>,
    pub tfidf_vector: Option<JsonValue>,
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

// ============================================================================
// 7. Intelligent Brain Models (Phase 16)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Pipeline {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub definition: JsonValue,
    pub diversity_score: f64,
    pub coverage_impact: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScenarioRule {
    pub id: i32,
    pub scenario_id: i32,
    pub pipeline_id: i32,
    
    // Contextual Targeting
    pub device_type: Option<String>,
    pub maturity_rating: Option<String>,
    
    pub priority: i32,
    pub condition: JsonValue,
    pub is_active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Consolidated view of a scenario and its primary strategy (pipeline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioWithStrategy {
    pub scenario: Scenario,
    pub pipeline: PipelineDefinition,
    pub is_active: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RuleSuggestion {
    pub id: i32,
    pub scenario_id: i32,
    pub suggested_pipeline_id: i32,
    pub suggested_condition: JsonValue,
    pub reasoning: Option<String>,
    pub confidence_score: Option<f64>,
    pub status: String,
    pub auto_apply_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
}

// ============================================================================
// 8. PageLayout (Dynamic UI Layouts - SDUI)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PageLayout {
    pub id: i32,
    pub page_slug: String,
    
    // Navigation Mesh
    pub is_landing: bool,
    pub nav_type: String,

    pub composition: JsonValue, // Array of structured objects
    pub device_type: Option<String>,
    pub maturity_rating: Option<String>,
    pub priority: i32,
    pub is_active: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// 9. DiscoveryConfig (Device-Specific Orchestration)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiscoveryConfig {
    pub device_type: String,
    pub initial_batch_size: i32,
    pub continuation_batch_size: i32,
    pub prewarm_lookahead: i32,
    pub ghost_ttl_seconds: i32,
    pub cache_ttl_seconds: i32,
    pub updated_at: DateTime<Utc>,
}
