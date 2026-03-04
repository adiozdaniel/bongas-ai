use std::sync::Arc;
pub use crate::engine::execution::core::execution_manager::service::ExecutionManager;
pub use crate::engine::governance::strategy::resolver::service::StrategyResolver;
pub use crate::engine::execution::cache::predictive_warmer::service::PredictiveWarmer;
pub use crate::engine::execution::cache::staging_manager::service::StagingManager;

use crate::pipeline::executor::service::PipelineExecutor;
use crate::cache::CacheManager;
use crate::ml::assets::loader::service::ModelLoader;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::ml::inference::features::service::FeatureStore;
use crate::db::repositories::feature_repository::service::FeatureRepository;
use crate::db::repositories::cache_repository::service::CacheRepository;
use crate::circuit_breaker::CircuitBreakerRegistry;

/// ⚡ THE STAGE: High-performance discovery execution.
pub struct ExecutionPillar {
    pub manager: Arc<ExecutionManager>,
    pub pipeline_executor: Arc<PipelineExecutor>,
    pub strategy_resolver: Arc<StrategyResolver>,
    pub staging_manager: Arc<StagingManager>,
    pub cache_manager: Arc<CacheManager>,
    pub model_loader: Arc<ModelLoader>,
    pub item_feature_service: Arc<ItemFeatureService>,
    pub feature_store: Arc<FeatureStore>,
    pub feature_repo: Arc<FeatureRepository>,
    pub cache_repo: Arc<CacheRepository>,
    pub circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    pub warmer: Arc<PredictiveWarmer>,
}

impl ExecutionPillar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        manager: Arc<ExecutionManager>,
        feature_repo: Arc<FeatureRepository>,
        cache_repo: Arc<CacheRepository>,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
        warmer: Arc<PredictiveWarmer>,
    ) -> Self {
        Self { 
            pipeline_executor: manager.pipeline_executor.clone(),
            strategy_resolver: manager.strategy_resolver.clone(),
            staging_manager: manager.staging_manager.clone(),
            cache_manager: manager.cache_manager.clone(),
            model_loader: manager.model_loader.clone(),
            item_feature_service: manager.item_feature_service.clone(),
            feature_store: manager.feature_store.clone(),
            manager, 
            feature_repo,
            cache_repo,
            circuit_breaker_registry,
            warmer,
        }
    }
}
