//! Execution context for pipeline stages with Netflix resilience dependencies.
//!
//! Every pipeline stage receives this context, which carries:
//! - User/request metadata
//! - Database pool, cache manager, model loader
//! - Feature store and embedding manager (resilient ML infrastructure)
//! - Performance analytics for per-stage metrics
//! - Pipeline resilience configuration

use std::sync::Arc;
use sqlx::PgPool;
use crate::analytics::types::PerformanceStats;
use crate::cache::CacheManager;
use crate::config::PipelineConfig;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::ml::model_loader::ModelLoader;
use crate::ml::feature_store::FeatureStore;
use crate::ml::embeddings::EmbeddingManager;

/// Execution context passed to all pipeline stages.
///
/// Stages use this context to access shared dependencies without
/// creating their own connections or duplicating feature-fetching logic.
pub struct ExecutionContext {
    // ── Request metadata ────────────────────────────────────────────────
    pub user_id: Option<i32>,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub request_id: String,

    // ── Core dependencies ───────────────────────────────────────────────
    pub db_pool: Arc<PgPool>,
    pub cache_manager: Arc<CacheManager>,
    pub model_loader: Arc<ModelLoader>,

    // ── Resilient data access ───────────────────────────────────────────
    /// Unified item/user feature service — pipeline stages should use this
    /// instead of querying db_pool directly.
    pub item_feature_service: Arc<ItemFeatureService>,

    // ── ML infrastructure (resilient) ───────────────────────────────────
    pub feature_store: Option<Arc<FeatureStore>>,
    pub embedding_manager: Option<Arc<EmbeddingManager>>,

    // ── Analytics ───────────────────────────────────────────────────────
    pub analytics: Option<Arc<PerformanceStats>>,

    // ── Pipeline config ─────────────────────────────────────────────────
    pub pipeline_config: Arc<PipelineConfig>,
}

impl ExecutionContext {
    /// Create a new execution context with core dependencies.
    pub fn new(
        user_id: Option<i32>,
        db_pool: Arc<PgPool>,
        cache_manager: Arc<CacheManager>,
        model_loader: Arc<ModelLoader>,
        item_feature_service: Arc<ItemFeatureService>,
        request_id: String,
    ) -> Self {
        Self {
            user_id,
            device_type: None,
            location: None,
            request_id,
            db_pool,
            cache_manager,
            model_loader,
            item_feature_service,
            feature_store: None,
            embedding_manager: None,
            analytics: None,
            pipeline_config: Arc::new(PipelineConfig::default()),
        }
    }

    // ── Builder methods ─────────────────────────────────────────────────

    pub fn with_device_type(mut self, device_type: String) -> Self {
        self.device_type = Some(device_type);
        self
    }

    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_feature_store(mut self, feature_store: Arc<FeatureStore>) -> Self {
        self.feature_store = Some(feature_store);
        self
    }

    pub fn with_embedding_manager(mut self, embedding_manager: Arc<EmbeddingManager>) -> Self {
        self.embedding_manager = Some(embedding_manager);
        self
    }

    pub fn with_analytics(mut self, analytics: Arc<PerformanceStats>) -> Self {
        self.analytics = Some(analytics);
        self
    }

    pub fn with_pipeline_config(mut self, config: Arc<PipelineConfig>) -> Self {
        self.pipeline_config = config;
        self
    }

    // ── Convenience accessors ───────────────────────────────────────────

    /// Record a response time metric if analytics is enabled.
    #[inline]
    pub fn record_stage_latency(&self, stage_name: &str, duration_ms: u64) {
        if let Some(ref a) = self.analytics {
            a.record_response_time(
                &format!("pipeline.stage.{}", stage_name),
                duration_ms,
            );
        }
    }

    /// Increment stage throughput counter if analytics is enabled.
    #[inline]
    pub fn record_stage_throughput(&self, stage_name: &str) {
        if let Some(ref a) = self.analytics {
            a.increment_throughput(&format!("pipeline.stage.{}", stage_name));
        }
    }

    /// Increment stage error counter if analytics is enabled.
    #[inline]
    pub fn record_stage_error(&self, stage_name: &str) {
        if let Some(ref a) = self.analytics {
            a.increment_error(&format!("pipeline.stage.{}", stage_name));
        }
    }
}
