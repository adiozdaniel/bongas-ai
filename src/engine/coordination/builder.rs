use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use anyhow::Result;
use tracing::info;
use std::collections::HashMap;
use arc_swap::ArcSwap;

use crate::AppConfig;
use crate::db::{ResilientPool, ScenarioRepository};
use crate::db::repositories::model_repository::service::ModelRepository;
use crate::db::repositories::feature_repository::service::FeatureRepository;
use crate::db::repositories::cache_repository::service::CacheRepository;
use crate::db::repositories::interaction_repository::service::InteractionRepository;
use crate::db::repositories::page_layout_repository::service::PageLayoutRepository;
use crate::db::repositories::discovery_repository::service::DiscoveryConfigRepository;
use crate::db::ItemFeatureService;
use crate::db::ResilientPoolConfig;

use crate::cache::manager::service::CacheManager;
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
use crate::engine::intelligence::workers::WorkersManager;
use crate::engine::intelligence::workers::tribe_orchestrator::service::TribeOrchestrator;
use crate::engine::intelligence::workers::regional_pulse::service::RegionalPulseWorker;
use crate::engine::intelligence::workers::fatigue_sync::service::FatigueSynchronizer;
use crate::engine::intelligence::workers::reasoning::service::ReasoningWorker;
use crate::engine::intelligence::workers::digest_worker::service::DigestWorker;
use crate::engine::intelligence::workers::sovereign_sight::service::SovereignSightWorker;
use crate::engine::governance::orchestration::manager::service::PagesManager;
use crate::engine::governance::strategy::resolver::service::StrategyResolver;
use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;
use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
use crate::engine::execution::cache::predictive_warmer::service::PredictiveWarmer;
use crate::ml::inference::features::service::FeatureStore;
use crate::experiments::coordinator::service::ExperimentCoordinator;
use crate::notification::{NotificationDispatcher, NotificationRepository};
use crate::notification::dispatcher::service::{KafkaNotifyAdaptor, PollingAdaptor, ResendNotifyAdaptor, NotificationAdaptor};
use crate::resilience::ResilienceMetricsCollector;
use crate::resilience::registry::MetricsRegistry;
use crate::resilience::ResilienceMetricsConfig;
use crate::circuit_breaker::CircuitBreakerRegistry;

/// 🎼 Discovery Symphony: The Grand Unified Orchestrator for Bongas-AI.
pub struct DiscoverySymphony {
    config: Arc<AppConfig>,
}

impl DiscoverySymphony {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    pub async fn assemble(&self) -> Result<Arc<BongasEngine>> {
        info!("🎼 Assembling the Bongas-AI Symphony 2.0...");

        let (shutdown_tx, _) = broadcast::channel(1);
        let circuit_breaker_registry = Arc::new(CircuitBreakerRegistry::new());
        let resilience_metrics_config = ResilienceMetricsConfig::default();
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(Arc::new(MetricsRegistry::new(resilience_metrics_config))));

        // ─── 1. CORE DATABASE PILLAR ─────────────────────────────────────────
        let pool_config = ResilientPoolConfig {
            url: self.config.database.url.clone().unwrap_or_default(),
            max_connections: self.config.database.max_connections,
            min_connections: self.config.database.min_connections,
            acquire_timeout: std::time::Duration::from_secs(self.config.database.connection_timeout),
            idle_timeout: std::time::Duration::from_secs(self.config.database.idle_timeout),
            max_lifetime: std::time::Duration::from_secs(self.config.database.max_lifetime),
            query_timeout: std::time::Duration::from_secs(30),
            max_concurrent_queries: 100, 
            failure_rate_threshold: 0.5,
            slow_call_rate_threshold: 0.5,
            slow_call_duration: std::time::Duration::from_secs(5),
        };

        let resilient_pool = Arc::new(ResilientPool::new(pool_config, circuit_breaker_registry.clone()).await?);

        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let interaction_repo = Arc::new(InteractionRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let layout_repo = Arc::new(PageLayoutRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let discovery_repo = Arc::new(DiscoveryConfigRepository::new(resilient_pool.clone(), resilience_metrics.clone()));

        // ─── 2. CACHING PILLAR ───────────────────────────────────────────────
        let cache_manager = Arc::new(CacheManager::new(
            self.config.redis.clone(),
            self.config.cache.clone(),
            Some(cache_repo.clone()),
        ).await?);

        let hot_registry = Arc::new(HotRegistry::new());

        // ─── 3. MACHINE LEARNING ─────────────────────────────────────────────
        let perf_stats = Arc::new(crate::analytics::types::PerformanceStats::new());

        let model_loader = Arc::new(ModelLoader::new(
            self.config.ml.model_path.clone(),
            model_repo.clone(),
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

        let feature_store = Arc::new(FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            self.config.ml.clone(),
            Some(perf_stats.clone()),
        ));

        let embedding_manager = Arc::new(EmbeddingManager::new(
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

        let security_manager = Arc::new(crate::security::SecurityManager::new(
            self.config.security.clone(),
            &self.config.server.environment,
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            None,
        ).await?);

        // ─── 4. SEARCH PILLAR (Meilisearch) ──────────────────────────────────
        let search_client = Arc::new(meilisearch_sdk::client::Client::new(
            self.config.search.host.clone(),
            Some(self.config.search.api_key.clone()),
        ).expect("Meilisearch client init failed"));

        let clickhouse_client = clickhouse::Client::default()
            .with_url(self.config.clickhouse.url.clone())
            .with_user(self.config.clickhouse.user.clone())
            .with_password(self.config.clickhouse.password.clone())
            .with_database(self.config.clickhouse.database.clone());

        let online_learning = Arc::new(OnlineLearningManager::new(
            &self.config.ml,
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ));

        let training_orchestrator = Arc::new(TrainingOrchestrator::new(
            self.config.ml.clone(),
            clickhouse_client.clone(),
            security_manager.clone(),
            model_loader.clone(),
        ));

        let training_pillar = Arc::new(TrainingPillar::new(
            online_learning,
            training_orchestrator,
            Arc::new(MlWorkerQueue::new(&self.config.ml, Some(perf_stats.clone()))),
        ));

        let ml_pillar = Arc::new(MlPillar::new(inference, training_pillar, assets));

        // ─── 4. EXECUTION PILLAR ─────────────────────────────────────────────
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            self.config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            Some(perf_stats.clone()),
        ));

        let staging_manager = Arc::new(StagingManager::new(
            cache_manager.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
        ));

        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));

        let experiment_coordinator = Arc::new(ExperimentCoordinator::new());

        let scenarios = Arc::new(RwLock::new(HashMap::new()));
        let linked_scenarios = Arc::new(ArcSwap::from_pointee(HashMap::new()));

        let execution_manager = Arc::new(ExecutionManager::new(
            pipeline_executor.clone(),
            Arc::new(StrategyResolver::new()),
            staging_manager.clone(),
            cache_manager.clone(),
            model_loader.clone(),
            item_feature_service.clone(),
            feature_store.clone(),
            Arc::new(self.config.ml.clone()),
            perf_stats.clone(),
            experiment_coordinator,
            Arc::new(crate::middlewares::MetricsCollector::default()),
            Some(Arc::new(clickhouse_client.clone())),
            Some(search_client.clone()),
            hot_registry.clone(),
            scenarios.clone(),
            linked_scenarios.clone(),
        ));

        let warmer = Arc::new(PredictiveWarmer::new(vec![], shutdown_tx.subscribe()));

        let execution = Arc::new(ExecutionPillar::new(
            execution_manager,
            feature_repo,
            cache_repo,
            interaction_repo.clone(),
            circuit_breaker_registry.clone(),
            warmer,
        ));

        let staleness_engine = Arc::new(StalenessEngine::new(
            staging_manager.clone(),
            item_feature_service.clone(),
        ));

        let monitoring = Arc::new(AnalyticsSidecar::new(
            clickhouse_client.clone(),
            resilient_pool.clone(),
            shutdown_tx.subscribe(),
        ));

        let tribe_orchestrator = Arc::new(TribeOrchestrator::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            self.config.ml.tribe_clustering_interval,
            self.config.ml.tribe_num_clusters,
        ));

        let hive_mind = Arc::new(HiveMindConnector::new(self.config.hive_mind.clone(), Some(resilient_pool.clone()), shutdown_tx.subscribe()));

        let regional_pulse_worker = Arc::new(RegionalPulseWorker::new(
            hive_mind.clone(),
            cache_manager.clone(),
            std::time::Duration::from_secs(3600), // Hourly scrape
        ));

        let fatigue_sync = Arc::new(FatigueSynchronizer::new(
            self.config.ml.fatigue_adaptor.clone(),
            cache_manager.clone(),
            Some(clickhouse_client.clone()),
        )?);

        let reasoning_worker = Arc::new(ReasoningWorker::new(
            hive_mind.clone(),
            cache_manager.clone(),
            item_feature_service.clone(),
            Some(clickhouse_client.clone()),
            std::time::Duration::from_secs(3600), // Hourly cycle
        ));

        let digest_worker = Arc::new(DigestWorker::new(
            std::time::Duration::from_secs(86400), // Daily cycle
        ));

        let search_sync_worker = Arc::new(crate::engine::intelligence::workers::search_sync::service::SearchSyncWorker::new(
            self.config.search.host.clone(),
            self.config.search.api_key.clone(),
            self.config.search.index_name.clone(),
            std::time::Duration::from_secs(3600), // Hourly sync
        ));

        let signal_decay_worker = Arc::new(crate::engine::intelligence::workers::signal_decay::service::SignalDecayWorker::new(
            clickhouse_client.clone(),
            std::time::Duration::from_secs(86400), // Daily decay
            self.config.ml.retention_days,
        ));

        let sovereign_sight_worker = Arc::new(SovereignSightWorker::new(
            resilient_pool.clone(),
            clickhouse_client.clone(),
            cache_manager.clone(),
            resilience_metrics.clone(),
            std::time::Duration::from_secs(3600), // Hourly audit pulse
        ));

        let workers = Arc::new(WorkersManager::new()
            .with_tribe_orchestrator(tribe_orchestrator)
            .with_regional_pulse(regional_pulse_worker)
            .with_fatigue_sync(fatigue_sync.clone())
            .with_reasoning(reasoning_worker)
            .with_digest(digest_worker.clone())
            .with_search_sync(search_sync_worker)
            .with_signal_decay(signal_decay_worker)
            .with_sovereign_sight(sovereign_sight_worker));

        let intelligence = Arc::new(IntelligencePillar::new(
            Arc::new(SuggestionsManager::new()),
            hive_mind,
            monitoring,
            staleness_engine,
            workers,
            fatigue_sync,
        ));

        // ─── 7. NOTIFICATIONS & SIDE-EFFECTS ──────────────────────────────────
        let notification_repo = Arc::new(NotificationRepository::new(
            resilient_pool.clone(),
            Some(clickhouse_client.clone()),
            resilience_metrics.clone(),
        ));

        let notification_adaptor: Arc<dyn NotificationAdaptor> = match self.config.notifications.adaptor {
            crate::config::types::notification::NotificationAdaptorKind::Kafka => Arc::new(KafkaNotifyAdaptor::new()),
            crate::config::types::notification::NotificationAdaptorKind::Resend => Arc::new(ResendNotifyAdaptor::new(self.config.notifications.resend.clone())),
            crate::config::types::notification::NotificationAdaptorKind::Polling => Arc::new(PollingAdaptor::new()),
        };

        let notification_dispatcher = Arc::new(NotificationDispatcher::new(
            notification_repo,
            notification_adaptor,
        ));

        // ─── 8. GOVERNANCE (PAGES & DISCOVERY) ────────────────────────────────
        let scenario_factory = Arc::new(ScenarioFactory::new(ScenarioRepository::new(resilient_pool.clone(), resilience_metrics.clone())));
        
        let scenarios_manager = Arc::new(ScenariosManager::new(
            scenario_factory.clone(),
            pipeline_executor.clone(),
            Arc::new(StrategyResolver::new()),
            staging_manager.clone(),
            100,
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

        // ─── 9. FINAL ENGINE ASSEMBLY ───────────────────────────────────────
        let engine = BongasEngine::new(crate::engine::coordination::service::EngineComponents {
            config: self.config.clone(),
            execution,
            governance,
            ml_pillar,
            intelligence,
            notifications: notification_dispatcher,
            cache: cache_manager,
            shutdown_tx,
            resilience_metrics,
        }).await?;

        let engine_arc = Arc::new(engine);
        engine_arc.intelligence.set_engine(Arc::downgrade(&engine_arc));

        info!("🎼 Symphony 2.0 fully assembled.");
        Ok(engine_arc)
    }
}
