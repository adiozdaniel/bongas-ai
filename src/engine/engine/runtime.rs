//! Engine runtime orchestration: bootstrap and initialization.

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry};
use crate::db::{ResilientPool, ResilientPoolConfig};
use crate::pipeline::executor::PipelineExecutor;
use crate::cache::{CacheManager, CacheConfig, HotRegistry};
use crate::ingestion::IngestionManager;
use crate::ingestion::metrics::IngestionMetrics;
use crate::ml::model_loader::ModelLoader;
use crate::ml::FeatureStore;
use crate::db::repositories::model_repository::ModelRepository;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::pages::PagesManager;
use crate::security::SecurityManager;
use crate::analytics::types::PerformanceStats;
use crate::experiments::ExperimentCoordinator;

use crate::engine::staging_manager::StagingManager;
use crate::engine::staleness_engine::StalenessEngine;
use crate::engine::scenario_factory::ScenarioFactory;
use crate::engine::config::EngineDependencies;
use crate::engine::strategy_resolver::StrategyResolver;
use crate::engine::analytics_sidecar::AnalyticsSidecar;
use crate::engine::hive_mind::HiveMindConnector;
use crate::engine::scenarios_manager::ScenariosManager;
use crate::engine::execution_manager::ExecutionManager;

use super::BongasEngine;

impl BongasEngine {
    /// Bootstrap the engine with all its dependencies
    pub async fn bootstrap(deps: EngineDependencies) -> Result<Arc<Self>> {
        info!("Bootstrapping BongasEngine Coordinator...");

        let db_pool = deps.db_pool.clone();
        let engine = Self::new(deps).await?;
        
        // Ensure migrations run before we start accepting requests
        info!("Ensuring database schema is up to date...");
        sqlx::migrate!("./migrations")
            .run(&db_pool)
            .await
            .context("Failed to run database migrations during bootstrap")?;

        info!("Loading initial scenarios from database...");
        if let Err(e) = engine.reload_scenarios().await {
            error!(error = %e, "Failed to load initial scenarios");
        }

        info!("Loading initial page layouts from database...");
        if let Err(e) = engine.pages.load_all_active().await {
            error!(error = %e, "Failed to load initial page layouts");
        }

        Ok(engine)
    }

    async fn new(deps: EngineDependencies) -> Result<Arc<Self>> {
        let EngineDependencies {
            config,
            db_pool,
            circuit_breaker_registry,
            metrics_collector,
        } = deps;

        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
            Arc::new(MetricsRegistry::new(config.resilience_metrics.clone())),
        ));
        let resilient_pool = Arc::new(ResilientPool::from_pool(
            db_pool.clone(),
            ResilientPoolConfig::default(),
            circuit_breaker_registry.clone(),
        )?);

        let security_manager = Arc::new(SecurityManager::new(config.security.clone(), &config.server.environment, circuit_breaker_registry.clone(), resilience_metrics.clone(), None).await?);
        let cache_manager = Arc::new(CacheManager::new(&config.redis.url, CacheConfig::default()).await?);
        let hot_registry = Arc::new(HotRegistry::new());
        let performance_stats = Arc::new(PerformanceStats::new());
        
        let staging_manager = Arc::new(StagingManager::new(cache_manager.clone(), resilient_pool.clone(), resilience_metrics.clone()));
        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));
        let staleness_engine = Arc::new(StalenessEngine::new(staging_manager.clone(), item_feature_service.clone()));

        let scenario_factory = Arc::new(ScenarioFactory::new(resilient_pool.clone(), resilience_metrics.clone()));
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            Some(performance_stats.clone()),
        ));
        let strategy_resolver = Arc::new(StrategyResolver::new());

        let scenarios_manager = Arc::new(ScenariosManager::new(
            scenario_factory,
            pipeline_executor.clone(),
            strategy_resolver.clone(),
            staging_manager.clone(),
            20
        ));

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_loader = Arc::new(ModelLoader::new(
            config.ml.model_path.to_str().unwrap_or("models"),
            model_repo.clone(),
            config.ml.clone(),
            resilience_metrics.clone(),
            None,
        ));

        let feature_store = Arc::new(FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            config.ml.clone(),
            None,
        ));

        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let page_layout_repo = Arc::new(PageLayoutRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let pages_manager = Arc::new(PagesManager::new(page_layout_repo));

        let ingestion_metrics = Arc::new(IngestionMetrics::new(Vec::new()));
        let ingestion_manager = IngestionManager::new(
            config.ingestion.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
            staleness_engine.clone(),
            circuit_breaker_registry.clone(),
            ingestion_metrics.clone(),
            None,
        );

        let execution_manager = Arc::new(ExecutionManager::new(
            pipeline_executor,
            strategy_resolver,
            staging_manager.clone(),
            cache_manager.clone(),
            model_loader.clone(),
            item_feature_service,
            feature_store,
            performance_stats,
            Arc::new(ExperimentCoordinator::new(config.experiments.clone())),
            metrics_collector,
            None,
            hot_registry,
            feature_repo,
            cache_repo,
            circuit_breaker_registry,
            scenarios_manager.scenarios.clone(),
            scenarios_manager.linked_scenarios.clone(),
        ));

        let ch_client = clickhouse::Client::default()
            .with_url(&config.clickhouse.url)
            .with_user(&config.clickhouse.user)
            .with_password(&config.clickhouse.password)
            .with_database(&config.clickhouse.database);

        let analytics_sidecar = Arc::new(AnalyticsSidecar::new(
            ch_client,
            resilient_pool.clone(),
            shutdown_tx.subscribe(),
        ));

        let hive_mind_connector = Arc::new(HiveMindConnector::new(
            config.hive_mind.clone(),
            resilient_pool.clone(),
            shutdown_tx.subscribe(),
        ));

        let engine = Arc::new(Self {
            config: config.clone(),
            resilience_metrics: resilience_metrics.clone(),
            scenarios: scenarios_manager,
            execution: execution_manager,
            staging_manager,
            staleness_engine,
            security_manager,
            cache_manager,
            ingestion_manager: Arc::new(RwLock::new(ingestion_manager)),
            analytics_sidecar: analytics_sidecar.clone(),
            hive_mind_connector: hive_mind_connector.clone(),
            pages: pages_manager,
            shutdown_tx,
        });

        analytics_sidecar.set_engine(Arc::downgrade(&engine));
        hive_mind_connector.set_engine(Arc::downgrade(&engine));

        Ok(engine)
    }
}
