use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};
use anyhow::Context;
use tracing::info;
use arc_swap::ArcSwap;

use crate::AppConfig;
use crate::error::AppResult;
use crate::engine::coordination::service::{BongasEngine, ScenarioDefinition};
use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceMetricsConfig};
use crate::cache::{CacheManager, CacheConfig, HotRegistry};
use crate::ingestion::IngestionManager;
use crate::security::SecurityManager;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::circuit_breaker::observer::traits::NoOpObserver;
use crate::analytics::types::PerformanceStats;
use crate::middlewares::metrics::MetricsCollector;
use crate::pipeline::types::models::ExecutablePipeline;

// Repositories & Mesh
use crate::db::{ResilientPool, ResilientPoolConfig};
use crate::db::repositories::feature_repository::service::FeatureRepository;
use crate::db::repositories::cache_repository::service::CacheRepository;
use crate::db::repositories::model_repository::service::ModelRepository;
use crate::db::repositories::interaction_repository::service::InteractionRepository;
use crate::db::repositories::scenario_repository::service::ScenarioRepository;
use crate::db::repositories::page_layout_repository::service::PageLayoutRepository;
use crate::db::repositories::discovery_repository::service::DiscoveryConfigRepository;

// Cortex (ML & Transformation)
use crate::ml::assets::loader::service::ModelLoader;
use crate::ml::inference::features::service::FeatureStore;

// Brain (The Three Pillars)
use crate::engine::intelligence::pillar::service::IntelligencePillar;
use crate::engine::intelligence::ai::suggestions_manager::service::SuggestionsManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::engine::intelligence::monitoring::analytics_sidecar::service::AnalyticsSidecar;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::intelligence::workers::workers_manager::service::WorkersManager;

use crate::engine::execution::pillar::service::ExecutionPillar;
use crate::engine::execution::core::execution_manager::service::ExecutionManager;
use crate::engine::governance::strategy::resolver::service::StrategyResolver;
use crate::engine::execution::cache::staging_manager::service::StagingManager;
use crate::engine::execution::cache::predictive_warmer::service::PredictiveWarmer;
use crate::pipeline::PipelineExecutor;

use crate::engine::governance::pillar::service::GovernancePillar;
use crate::engine::governance::orchestration::manager::service::PagesManager;
use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;
use crate::engine::governance::strategy::loader::service::ScenarioLoader;
use crate::experiments::coordinator::service::ExperimentCoordinator;

/// 🏗️ DiscoverySymphony: The Grand Factory for BONGAS-AI.
pub struct DiscoverySymphony {
    config: Arc<AppConfig>,
}

impl DiscoverySymphony {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    pub async fn assemble(self) -> AppResult<Arc<BongasEngine>> {
        info!("🎼 Starting DiscoverySymphony Assembly...");

        // ─── 1. FOUNDATIONAL LAYER ───────────────────────────────────────────
        let (shutdown_tx, _) = broadcast::channel(100);
        let circuit_breaker_registry = Arc::new(CircuitBreakerRegistry::new());
        let observer = Arc::new(NoOpObserver);
        
        let resilience_config = ResilienceMetricsConfig::default();
        let metrics_registry = Arc::new(MetricsRegistry::new(resilience_config));
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(metrics_registry));
        let performance_stats = Arc::new(PerformanceStats::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        let hot_registry = Arc::new(HotRegistry::new());

        // ─── 2. DATA MESH ────────────────────────────────────────────────────
        let db_url = self.config.database.url.as_deref()
            .ok_or_else(|| anyhow::anyhow!("DATABASE_URL not configured"))?;
        
        let db_pool = sqlx::PgPool::connect(db_url).await
            .context("Failed to connect to database mesh")?;
        
        let resilient_pool = Arc::new(ResilientPool::from_pool(
            db_pool,
            ResilientPoolConfig::default(),
            circuit_breaker_registry.clone(),
        ).map_err(|e| anyhow::anyhow!("Pool config error: {}", e))?);

        let cache_config = CacheConfig::default();
        let cache_manager = Arc::new(CacheManager::new(&self.config.redis.url, cache_config).await
            .context("Failed to initialize multi-tier cache")?);

        // Repositories
        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let scenario_repo = Arc::new(ScenarioRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let layout_repo = Arc::new(PageLayoutRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let discovery_repo = Arc::new(DiscoveryConfigRepository::new(resilient_pool.clone(), resilience_metrics.clone()));

        // ─── 3. CORTEX & SECURITY ────────────────────────────────────────────
        let security = SecurityManager::new(
            self.config.security.clone(),
            &self.config.server.environment,
            circuit_breaker_registry.clone(),
            observer.clone(),
            Some(performance_stats.clone()),
        ).await.map_err(|e| anyhow::anyhow!("Security init error: {:?}", e))?;
        let security = Arc::new(security);

        let model_loader = Arc::new(ModelLoader::new(
            "models",
            model_repo,
            self.config.ml.clone(),
            resilience_metrics.clone(),
            None, // Fix: ModelLoader doesn't take ShutdownTx
        ));

        let feature_store = Arc::new(FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            self.config.ml.clone(),
            None, // Fix: FeatureStore doesn't take ShutdownTx
        ));

        // ─── 4. BRAIN PILLARS (The Triple Play) ──────────────────────────────
        
        // Component: Pipeline & Execution
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            self.config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            observer.clone(),
            Some(performance_stats.clone()),
        ));

        let staging_manager = Arc::new(StagingManager::new(
            cache_manager.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
        ));

        let strategy_resolver = Arc::new(StrategyResolver::new());

        let item_feature_service = Arc::new(crate::db::ItemFeatureService::new(
            resilient_pool.clone(),
            resilience_metrics.clone(),
        ));

        let experiment_coordinator = Arc::new(ExperimentCoordinator::new(
            self.config.experiments.clone(),
        ));

        let scenarios_map = Arc::new(RwLock::new(HashMap::<String, ScenarioDefinition>::new()));
        let linked_scenarios = Arc::new(ArcSwap::from_pointee(HashMap::<String, Arc<ExecutablePipeline>>::new()));

        let execution_manager = Arc::new(ExecutionManager::new(
            pipeline_executor.clone(),
            strategy_resolver.clone(),
            staging_manager.clone(),
            cache_manager.clone(),
            model_loader.clone(),
            item_feature_service.clone(),
            feature_store.clone(),
            performance_stats.clone(),
            experiment_coordinator.clone(),
            metrics_collector.clone(),
            None, // clickhouse
            hot_registry.clone(),
            scenarios_map.clone(),
            linked_scenarios.clone(),
        ));

        let warmer = Arc::new(PredictiveWarmer::new(
            vec!["trending".into()],
            shutdown_tx.subscribe(),
        ));

        let execution = Arc::new(ExecutionPillar::new(
            execution_manager.clone(),
            feature_repo,
            cache_repo,
            circuit_breaker_registry.clone(),
            warmer.clone(),
        ));

        // Component: Governance
        let scenario_factory = Arc::new(ScenarioFactory::new(
            resilient_pool.clone(),
            resilience_metrics.clone(),
        ));

        let _scenario_loader = Arc::new(ScenarioLoader::new(scenario_repo));

        let scenarios = Arc::new(ScenariosManager::new(
            scenario_factory.clone(),
            pipeline_executor.clone(),
            strategy_resolver.clone(),
            staging_manager.clone(),
            100, // max scenarios
        ));

        let interaction_repo = Arc::new(InteractionRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let pages = Arc::new(PagesManager::new(
            layout_repo,
            interaction_repo.clone(),
            feature_store.clone(),
            100, // cache size
        ));

        let governance = Arc::new(GovernancePillar::new(
            pages,
            scenarios,
            scenario_factory,
            discovery_repo,
        ));

        // Hydrate discovery configs
        governance.reload_discovery_configs().await.context("Failed to hydrate discovery configurations")?;

        // Component: Intelligence
        let intelligence = Arc::new(IntelligencePillar::new(
            Arc::new(SuggestionsManager::new()),
            Arc::new(HiveMindConnector::new(
                self.config.hive_mind.clone(),
                resilient_pool.clone(),
                shutdown_tx.subscribe(),
            )),
            Arc::new(AnalyticsSidecar::new(
                clickhouse::Client::default(),
                resilient_pool.clone(),
                shutdown_tx.subscribe(),
            )),
            Arc::new(StalenessEngine::new(
                staging_manager.clone(),
                item_feature_service.clone(),
            )),
            Arc::new(WorkersManager::new()),
        ));

        // ─── 5. INGESTION BACKBONE ───────────────────────────────────────────
        let staleness_engine = intelligence.staleness.clone();
        let pages_manager = governance.orchestration.clone();
        
        let ingestion_manager = IngestionManager::bootstrap(
            resilient_pool.clone(),
            None,
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            staleness_engine,
            pages_manager,
            self.config.ingestion.kafka.brokers.clone(),
        ).await.context("Failed to bootstrap Ingestion Symphony")?;

        let ingestion = Arc::new(RwLock::new(ingestion_manager));

        // ─── 6. ASSEMBLE ENGINE ──────────────────────────────────────────────
        let engine = Arc::new(BongasEngine {
            config: self.config,
            resilience_metrics,
            execution,
            governance,
            intelligence,
            security,
            cache: cache_manager,
            ingestion,
            shutdown_tx,
        });

        // ─── 7. FINAL WIRING ─────────────────────────────────────────────────
        warmer.set_engine(engine.clone()).await;

        info!("✅ DiscoverySymphony assembly complete.");
        Ok(engine)
    }
}
