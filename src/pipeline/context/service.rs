//! Execution context for pipeline stages with Netflix resilience dependencies.
//!
//! Every pipeline stage receives this context, which carries:
//! - User/request metadata
//! - Database pool, cache manager, model loader
//! - Feature store and embedding manager (resilient ML infrastructure)
//! - Performance analytics for per-stage metrics
//! - Pipeline resilience configuration

use std::collections::HashMap;
use std::sync::Arc;
use crate::analytics::types::PerformanceStats;
use crate::cache::{CacheManager, HotRegistry};
use crate::config::{PipelineConfig, MlConfig};
use crate::db::ItemFeatureService;
use crate::ml::assets::loader::service::ModelLoader;
use crate::ml::inference::features::service::FeatureStore;
use crate::ml::inference::embeddings::service::EmbeddingManager;

/// Execution context passed to all pipeline stages.
///
/// Stages use this context to access shared dependencies without
/// creating their own connections or duplicating feature-fetching logic.
#[derive(Clone)]
pub struct ExecutionContext {
    // ── Request metadata ────────────────────────────────────────────────
    pub user_id: Option<i32>,
    pub profile_id: Option<String>,
    pub maturity_rating: Option<String>,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub request_id: String,
    pub request_time: chrono::DateTime<chrono::Utc>,

    // ── Core dependencies ───────────────────────────────────────────────
    pub cache_manager: Arc<CacheManager>,
    pub model_loader: Arc<ModelLoader>,
    pub hot_registry: Option<Arc<HotRegistry>>,

    // ── Resilient data access ───────────────────────────────────────────
    /// Unified item/user feature service — pipeline stages should use this
    /// instead of querying db_pool directly.
    pub item_feature_service: Arc<ItemFeatureService>,

    /// ClickHouse client for analytics-heavy retrieval stages.
    pub clickhouse_client: Option<Arc<clickhouse::Client>>,

    // ── ML infrastructure (resilient) ───────────────────────────────────
    pub feature_store: Arc<FeatureStore>,
    pub embedding_manager: Option<Arc<EmbeddingManager>>,

    // ── Analytics ───────────────────────────────────────────────────────
    pub analytics: Option<Arc<PerformanceStats>>,

    // ── Experiments ─────────────────────────────────────────────────────
    pub experiment_overrides: HashMap<String, serde_json::Value>,

    // ── Pipeline config ─────────────────────────────────────────────────
    pub pipeline_config: Arc<PipelineConfig>,
    pub ml_config: Arc<MlConfig>,
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("user_id", &self.user_id)
            .field("device_type", &self.device_type)
            .field("location", &self.location)
            .field("request_id", &self.request_id)
            .finish_non_exhaustive()
    }
}

impl ExecutionContext {
    /// Create a new execution context with core dependencies.
    pub fn new(
        user_id: Option<i32>,
        cache_manager: Arc<CacheManager>,
        model_loader: Arc<ModelLoader>,
        item_feature_service: Arc<ItemFeatureService>,
        feature_store: Arc<FeatureStore>,
        request_id: String,
    ) -> Self {
        Self {
            user_id,
            profile_id: None,
            maturity_rating: None,
            device_type: None,
            location: None,
            request_id,
            request_time: chrono::Utc::now(),
            cache_manager,
            model_loader,
            hot_registry: None,
            item_feature_service,
            clickhouse_client: None,
            feature_store,
            embedding_manager: None,
            analytics: None,
            experiment_overrides: HashMap::new(),
            pipeline_config: Arc::new(PipelineConfig::default()),
            ml_config: Arc::new(MlConfig::default()),
        }
    }

    /// Create a test context with minimal valid dependencies for benchmarks.
    pub async fn test_context() -> Self {
        use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceMetricsConfig};
        use crate::db::{ResilientPool, ResilientPoolConfig};
        use crate::cache::{CacheManager, CacheConfig};
        use crate::db::repositories::model_repository::service::ModelRepository;
        use crate::circuit_breaker::CircuitBreakerRegistry;
        
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
            Arc::new(MetricsRegistry::new(ResilienceMetricsConfig::default())),
        ));
        
        // Use a dummy pool that won't connect unless used
        let db_pool = sqlx::PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
        let circuit_breaker_registry = Arc::new(CircuitBreakerRegistry::default());
        
        let resilient_pool = Arc::new(ResilientPool::from_pool(
            db_pool,
            ResilientPoolConfig::default(),
            circuit_breaker_registry.clone(),
        ).unwrap());

        let cache_config = CacheConfig {
            l1_enabled: true,
            l2_enabled: false, // Disable Redis for benchmarks
            ..Default::default()
        };
        let redis_config = crate::config::RedisConfig {
            url: "redis://localhost".to_string(),
            ..Default::default()
        };
        let cache_manager = Arc::new(CacheManager::new(redis_config, cache_config, None).await.unwrap());
        
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_loader = Arc::new(ModelLoader::new(
            "models",
            model_repo,
            crate::config::MlConfig::default(),
            resilience_metrics.clone(),
            None,
        ));

        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));
        let feature_store = Arc::new(FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            crate::config::MlConfig::default(),
            None,
        ));

        Self::new(
            Some(123),
            cache_manager,
            model_loader,
            item_feature_service,
            feature_store,
            "bench-request".to_string(),
        )
        .with_request_time(chrono::Utc::now())
        .with_hot_registry(Arc::new(HotRegistry::new()))
        .with_analytics(Arc::new(PerformanceStats::new()))
        .with_ml_config(Arc::new(MlConfig::default()))
    }

    // ── Builder methods ─────────────────────────────────────────────────

    pub fn with_request_time(mut self, request_time: chrono::DateTime<chrono::Utc>) -> Self {
        self.request_time = request_time;
        self
    }

    pub fn with_hot_registry(mut self, hot_registry: Arc<HotRegistry>) -> Self {
        self.hot_registry = Some(hot_registry);
        self
    }

    pub fn with_clickhouse_client(mut self, client: Arc<clickhouse::Client>) -> Self {
        self.clickhouse_client = Some(client);
        self
    }

    pub fn with_profile_id(mut self, profile_id: String) -> Self {
        self.profile_id = Some(profile_id);
        self
    }

    pub fn with_maturity_rating(mut self, maturity_rating: String) -> Self {
        self.maturity_rating = Some(maturity_rating);
        self
    }

    pub fn with_device_type(mut self, device_type: String) -> Self {
        self.device_type = Some(device_type);
        self
    }

    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
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

    pub fn with_experiment_overrides(mut self, overrides: HashMap<String, serde_json::Value>) -> Self {
        self.experiment_overrides = overrides;
        self
    }

    pub fn with_pipeline_config(mut self, config: Arc<PipelineConfig>) -> Self {
        self.pipeline_config = config;
        self
    }

    pub fn with_ml_config(mut self, config: Arc<MlConfig>) -> Self {
        self.ml_config = config;
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
