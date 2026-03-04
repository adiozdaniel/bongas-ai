use std::sync::Arc;
pub use crate::engine::execution::core::execution_manager::service::ExecutionManager;
pub use crate::engine::governance::strategy::resolver::service::StrategyResolver;
pub use crate::engine::execution::cache::predictive_warmer::service::PredictiveWarmer;
pub use crate::engine::execution::cache::staging_manager::service::StagingManager;

use crate::pipeline::executor::PipelineExecutor;
use crate::cache::CacheManager;
use crate::ml::model_loader::ModelLoader;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::ml::FeatureStore;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
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
        pipeline_executor: Arc<PipelineExecutor>,
        strategy_resolver: Arc<StrategyResolver>,
        staging_manager: Arc<StagingManager>,
        cache_manager: Arc<CacheManager>,
        model_loader: Arc<ModelLoader>,
        item_feature_service: Arc<ItemFeatureService>,
        feature_store: Arc<FeatureStore>,
        feature_repo: Arc<FeatureRepository>,
        cache_repo: Arc<CacheRepository>,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
        warmer: Arc<PredictiveWarmer>,
    ) -> Self {
        Self { 
            manager, 
            pipeline_executor,
            strategy_resolver,
            staging_manager,
            cache_manager,
            model_loader,
            item_feature_service,
            feature_store,
            feature_repo,
            cache_repo,
            circuit_breaker_registry,
            warmer,
        }
    }
}
