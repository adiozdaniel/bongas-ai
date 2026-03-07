use std::sync::Arc;
use tokio::sync::broadcast;
use anyhow::{Result, Context};
use tracing::info;

use crate::AppConfig;
use crate::db::ResilientPool;
use crate::db::repositories::model_repository::service::ModelRepository;
use crate::db::repositories::feature_repository::service::FeatureRepository;
use crate::db::repositories::cache_repository::service::CacheRepository;
use crate::db::repositories::interaction_repository::service::InteractionRepository;
use crate::db::repositories::page_layout_repository::service::PageLayoutRepository;
use crate::db::repositories::discovery_repository::service::DiscoveryConfigRepository;
use crate::db::ItemFeatureService;
use crate::db::ResilientPoolConfig;

use crate::cache::manager::service::CacheManager;
use crate::cache::config::models::CacheConfig;
use crate::cache::hot_registry::service::HotRegistry;
use crate::ml::assets::loader::service::ModelLoader;
use crate::ml::assets::registry::service::VersionedModelRegistry;
use crate::ml::assets::pillar::service::AssetsPillar;
use crate::ml::inference::pillar::service::InferencePillar;
use crate::ml::inference::embeddings::service::EmbeddingManager;
use crate::ml::inference::onnx::service::OnnxInferenceEngine;
use crate::ml::training::pillar::service::TrainingPillar;
use crate::ml::training::online::service::OnlineLearningManager;
use crate::ml::training::orchestration::service::TrainingOrchestrator;
use crate::ml::training::workers::service::MlWorkerQueue;
use crate::ml::coordination::service::MlPillar;

use crate::pipeline::executor::service::PipelineExecutor;
use crate::engine::coordination::service::BongasEngine;
use crate::engine::governance::pillar::service::GovernancePillar;
use crate::engine::execution::pillar::service::ExecutionPillar;
use crate::engine::execution::core::execution_manager::service::ExecutionManager;
use crate::engine::intelligence::pillar::service::IntelligencePillar;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::intelligence::monitoring::analytics_sidecar::service::AnalyticsSidecar;
use crate::engine::execution::cache::staging_manager::service::StagingManager;
use crate::engine::intelligence::ai::suggestions_manager::service::SuggestionsManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::engine::intelligence::workers::workers_manager::service::WorkersManager;
use crate::engine::governance::orchestration::manager::service::PagesManager;
use crate::engine::governance::strategy::resolver::service::StrategyResolver;
use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;
use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
use crate::engine::execution::cache::predictive_warmer::service::PredictiveWarmer;
use crate::ml::inference::features::service::FeatureStore;
use crate::experiments::coordinator::service::ExperimentCoordinator;
use crate::resilience::ResilienceMetricsCollector;
use crate::resilience::registry::MetricsRegistry;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::analytics::types::PerformanceStats;
use crate::middlewares::MetricsCollector;

/// Orchestrates the assembly of the Bongas-AI Engine.
pub struct DiscoverySymphony {
    config: Arc<AppConfig>,
}

impl DiscoverySymphony {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Assemble the complete engine with all pillars and shared infrastructure.
    pub async fn assemble(&self) -> Result<Arc<BongasEngine>> {
        info!("🎼 Assembling Symphony 2.0 components...");

        // ─── 1. CORE INFRASTRUCTURE ─────────────────────────────────────────
        let (shutdown_tx, _) = broadcast::channel(1);
        let metrics_registry = Arc::new(MetricsRegistry::new(crate::resilience::ResilienceMetricsConfig::default()));
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(metrics_registry.clone()));
        let circuit_breaker_registry = Arc::new(CircuitBreakerRegistry::with_observer(resilience_metrics.clone()));
        let perf_stats = Arc::new(PerformanceStats::new());

        // ─── 2. DATA MESH ────────────────────────────────────────────────────
        let db_url = self.config.database.url.as_deref()
            .ok_or_else(|| anyhow::anyhow!("DATABASE_URL not configured"))?;
        
        let db_config = ResilientPoolConfig::new(db_url)
            .with_max_connections(self.config.database.max_connections)
            .with_bulkhead_size(self.config.database.max_connections as usize);

        let resilient_pool = Arc::new(ResilientPool::new(
            db_config,
            circuit_breaker_registry.clone(),
        ).await.map_err(|e| anyhow::anyhow!("Pool config error: {}", e))?);

        // Repositories
        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let interaction_repo = Arc::new(InteractionRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let layout_repo = Arc::new(PageLayoutRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let discovery_repo = Arc::new(DiscoveryConfigRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));

        let cache_config = CacheConfig::default();
        let cache_manager = Arc::new(CacheManager::new(self.config.redis.clone(), cache_config, Some(cache_repo.clone())).await
            .context("Failed to initialize multi-tier cache")?);

        // ─── 3. MACHINE LEARNING ─────────────────────────────────────────────
        let model_loader = Arc::new(ModelLoader::new(
            self.config.ml.model_path.to_str().unwrap_or("models"),
            model_repo,
            self.config.ml.clone(),
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ));

        let model_registry = Arc::new(VersionedModelRegistry::new(
            self.config.ml.clone(),
            Some(perf_stats.clone()),
            circuit_breaker_registry.clone(),
        ));

        let assets = Arc::new(AssetsPillar::new(model_registry, model_loader.clone()));

        let dummy_engine = Arc::new(OnnxInferenceEngine::with_defaults(
            crate::circuit_breaker::observer::CircuitBreakerId::new("dummy")
        ));

        let embedding_manager = Arc::new(EmbeddingManager::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            self.config.ml.clone(),
            Some(perf_stats.clone()),
        ));

        let feature_store = Arc::new(FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            self.config.ml.clone(),
            Some(perf_stats.clone()),
        ));

        let inference = Arc::new(InferencePillar::new(
            dummy_engine,
            embedding_manager,
            feature_store.clone(),
        ));

        let online_learning = Arc::new(OnlineLearningManager::new(
            &self.config.ml,
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ));

        let security_manager = Arc::new(crate::security::SecurityManager::new(
            self.config.security.clone(),
            &self.config.server.environment,
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ).await.unwrap());

        let training_orch = Arc::new(TrainingOrchestrator::new(
            self.config.ml.clone(),
            clickhouse::Client::default(),
            security_manager,
            model_loader.clone(),
        ));

        let ml_workers = Arc::new(MlWorkerQueue::new(&self.config.ml, Some(perf_stats.clone())));

        let training = Arc::new(TrainingPillar::new(
            online_learning,
            training_orch,
            ml_workers,
        ));

        let ml_pillar = Arc::new(MlPillar {
            inference,
            training,
            assets,
        });

        // ─── 4. PIPELINE ORCHESTRATION ──────────────────────────────────────
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            self.config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ));

        // ─── 5. INTELLIGENCE & MONITORING ────────────────────────────────────
        let staging_manager = Arc::new(StagingManager::new(
            cache_manager.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
        ));

        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));

        let staleness_engine = Arc::new(StalenessEngine::new(
            staging_manager.clone(),
            item_feature_service.clone(),
        ));

        let clickhouse_client = clickhouse::Client::default()
            .with_url(self.config.clickhouse.url.clone())
            .with_user(self.config.clickhouse.user.clone())
            .with_password(self.config.clickhouse.password.clone())
            .with_database(self.config.clickhouse.database.clone());

        let monitoring = Arc::new(AnalyticsSidecar::new(
            clickhouse_client.clone(),
            resilient_pool.clone(),
            shutdown_tx.subscribe(),
        ));

        let intelligence = Arc::new(IntelligencePillar::new(
            Arc::new(SuggestionsManager::new()),
            Arc::new(HiveMindConnector::new(self.config.hive_mind.clone(), resilient_pool.clone(), shutdown_tx.subscribe())),
            monitoring,
            staleness_engine,
            Arc::new(WorkersManager::new()),
        ));

        // ─── 6. GOVERNANCE (PAGES & DISCOVERY) ────────────────────────────────
        let strategy_resolver = Arc::new(StrategyResolver::new());
        let scenario_factory = Arc::new(ScenarioFactory::new(resilient_pool.clone(), resilience_metrics.clone()));
        
        let scenarios_manager = Arc::new(ScenariosManager::new(
            scenario_factory.clone(),
            pipeline_executor.clone(),
            strategy_resolver.clone(),
            staging_manager.clone(),
            100, // max scenarios
        ));

        let pages_manager = Arc::new(PagesManager::new(
            layout_repo,
            interaction_repo.clone(),
            feature_store.clone(),
            scenarios_manager.clone(),
            1000,
        ));

        let governance = Arc::new(GovernancePillar::new(
            pages_manager.clone(),
            scenarios_manager.clone(),
            scenario_factory,
            discovery_repo,
        ));

        // ─── 7. EXECUTION ENGINE ─────────────────────────────────────────────
        let execution_manager = Arc::new(ExecutionManager::new(
            pipeline_executor,
            strategy_resolver,
            staging_manager,
            cache_manager.clone(),
            model_loader.clone(),
            item_feature_service,
            feature_store,
            perf_stats,
            Arc::new(ExperimentCoordinator::new(self.config.experiments.clone())),
            Arc::new(MetricsCollector::new()),
            Some(Arc::new(clickhouse_client)),
            Arc::new(HotRegistry::new()),
            scenarios_manager.scenarios.clone(),
            scenarios_manager.linked_scenarios.clone(),
        ));

        let execution = Arc::new(ExecutionPillar::new(
            execution_manager,
            feature_repo,
            cache_repo,
            interaction_repo,
            circuit_breaker_registry,
            Arc::new(PredictiveWarmer::new(vec![], shutdown_tx.subscribe())),
        ));

        // ─── 8. FINAL ENGINE ASSEMBLY ───────────────────────────────────────
        let engine = BongasEngine::new(
            self.config.clone(),
            execution,
            governance,
            ml_pillar,
            intelligence,
            cache_manager,
            shutdown_tx,
            resilience_metrics,
        ).await.map_err(|e| anyhow::anyhow!("Engine assembly failed: {}", e))?;

        let engine_arc = Arc::new(engine);

        // ─── 9. FINAL WIRING ────────────────────────────────────────────────
        // Inject engine weak references into pillars that need them
        engine_arc.intelligence.set_engine(Arc::downgrade(&engine_arc));

        info!("🎼 Symphony 2.0 fully assembled.");
        Ok(engine_arc)
    }
}
