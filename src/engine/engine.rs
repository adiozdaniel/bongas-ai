use anyhow::{Result, Context};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};
use arc_swap::ArcSwap;

use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceConfig};
use crate::db::{ResilientPool, ResilientPoolConfig};
use crate::db::models::PipelineDefinition;
use crate::AppConfig;
use crate::pipeline::executor::PipelineExecutor;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::{ScoredItem, ExecutablePipeline};
use crate::cache::{CacheManager, CacheConfig, CacheWarmer, CacheMetricsSnapshot, HotRegistry, HotItem};
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::ingestion::IngestionManager;
use crate::ingestion::metrics::IngestionMetrics;
use crate::ml::model_loader::ModelLoader;
use crate::ml::FeatureStore;
use crate::db::repositories::model_repository::ModelRepository;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::security::SecurityManager;
use crate::middlewares::MetricsCollector;
use crate::analytics::types::PerformanceStats;
use crate::experiments::ExperimentCoordinator;

use super::staging_manager::StagingManager;
use super::staleness_engine::{StalenessEngine, UserEvent};
use super::scenario_factory::ScenarioFactory;
use super::predictive_warmer::PredictiveWarmer;
use super::config::EngineDependencies;
use super::strategy_resolver::StrategyResolver;

/// Central orchestrator for BONGAS-AI
pub struct BongasEngine {
    // Configuration
    pub(crate) config: Arc<AppConfig>,

    // Scenario management
    pub(crate) scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
    pub(crate) linked_scenarios: Arc<ArcSwap<HashMap<String, Arc<ExecutablePipeline>>>>,
    pub(crate) scenario_factory: Arc<ScenarioFactory>,

    // Pipeline execution
    pub(crate) pipeline_executor: Arc<PipelineExecutor>,
    pub(crate) strategy_resolver: Arc<StrategyResolver>,

    // Caching & staging
    pub(crate) staging_manager: Arc<StagingManager>,
    pub(crate) staleness_engine: Arc<StalenessEngine>,
    pub(crate) hot_registry: Arc<HotRegistry>,

    // ML Model Management
    pub(crate) model_loader: Arc<ModelLoader>,
    pub(crate) training_orchestrator: Arc<crate::ml::TrainingOrchestrator>,

    // Repositories & services
    pub(crate) item_feature_service: Arc<ItemFeatureService>,
    pub(crate) feature_store: Arc<FeatureStore>,
    pub(crate) feature_repo: Arc<FeatureRepository>,
    pub(crate) cache_repo: Arc<CacheRepository>,
    pub(crate) clickhouse: Option<Arc<clickhouse::Client>>,

    // Ingestion
    pub(crate) ingestion_manager: Arc<RwLock<IngestionManager>>,
    pub(crate) ingestion_metrics: Arc<IngestionMetrics>,

    // Experiments
    pub(crate) experiment_coordinator: Arc<ExperimentCoordinator>,

    // Governance
    pub(crate) max_active_scenarios: Arc<AtomicUsize>,

    // Security
    pub(crate) security_manager: Arc<SecurityManager>,

    // Resilience
    pub(crate) circuit_breaker_registry: Arc<CircuitBreakerRegistry>,

    // Dependencies
    pub(crate) cache_manager: Arc<CacheManager>,
    pub(crate) metrics_collector: Arc<MetricsCollector>,
    pub(crate) performance_stats: Arc<PerformanceStats>,

    // Shutdown signal
    pub(crate) shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub pipeline: PipelineDefinition,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
    pub initial_display_limit: i32,
    pub scope: serde_json::Value,
    pub linked_pipeline: Option<Arc<ExecutablePipeline>>,
}

impl BongasEngine {
    /// Bootstrap the engine with all its dependencies
    pub async fn bootstrap(deps: EngineDependencies) -> Result<Arc<Self>> {
        info!("Bootstrapping BongasEngine...");

        let engine = Self::new(deps).await?;
        
        // Load initial scenarios
        engine.reload_scenarios().await?;

        // Start ingestion if enabled
        if engine.config.ingestion.kafka.enabled || 
           engine.config.ingestion.api.enabled || 
           engine.config.ingestion.clickhouse.enabled {
            engine.start_ingestion(&engine.config.ingestion).await?;
        }

        // Start cache warming if enabled
        let cache_config = CacheConfig::default();
        if cache_config.warming_enabled {
            engine.clone().start_cache_warming(
                cache_config.warm_scenarios.clone(),
                cache_config.warming_interval,
            );

            // Phase 6: Start Predictive Warmer
            let shutdown_rx = engine.shutdown_tx.subscribe();
            let predictive_warmer = PredictiveWarmer::new(
                engine.clone(),
                cache_config.warm_scenarios,
                shutdown_rx,
            );
            tokio::spawn(async move {
                predictive_warmer.start().await;
            });
        }

        // Phase 15: Run One-Shot Harvest for first launch
        let engine_for_harvest = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_for_harvest.training_orchestrator.run_one_shot_harvest().await {
                error!(error = %e, "One-Shot Harvest failed");
            }
        });

        info!("BongasEngine bootstrapped successfully");
        Ok(engine)
    }

    /// Create new BongasEngine with highly concurrent initialization.
    async fn new(deps: EngineDependencies) -> Result<Arc<Self>> {
        let EngineDependencies {
            config,
            db_pool,
            circuit_breaker_registry,
            metrics_collector,
        } = deps;

        info!("Initializing BongasEngine components concurrently...");

        // 1. Shared resilience infrastructure
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
            Arc::new(MetricsRegistry::new(ResilienceConfig::default())),
        ));
        let resilient_pool = Arc::new(ResilientPool::from_pool(
            db_pool.clone(),
            ResilientPoolConfig::default(),
            circuit_breaker_registry.clone(),
        )?);

        // 2. Concurrently initialize independent subsystems
        
        // Task A: Security Manager
        let sec_config = config.security.clone();
        let sec_registry = circuit_breaker_registry.clone();
        let sec_observer = resilience_metrics.clone();
        let security_manager_fut: tokio::task::JoinHandle<Result<SecurityManager, anyhow::Error>> = tokio::spawn(async move {
            info!("Initializing SecurityManager...");
            SecurityManager::new(sec_config, sec_registry, sec_observer, None)
                .context("Failed to create SecurityManager")
        });

        // Task B: Cache Manager (Redis)
        let redis_url = config.redis.url.clone();
        let cache_config_val = CacheConfig::default();
        let cache_manager_fut: tokio::task::JoinHandle<Result<CacheManager, anyhow::Error>> = tokio::spawn(async move {
            info!("Establishing connection to Redis cache...");
            CacheManager::new(&redis_url, cache_config_val).await
                .context("Failed to create CacheManager")
        });

        // Task C: ClickHouse Client
        let ch_config = config.clickhouse.clone();
        let clickhouse_fut: tokio::task::JoinHandle<Result<Option<Arc<clickhouse::Client>>, anyhow::Error>> = tokio::spawn(async move {
            if !ch_config.url.is_empty() {
                info!("Establishing connection to ClickHouse at {}...", ch_config.url);
                let client = clickhouse::Client::default()
                    .with_url(&ch_config.url)
                    .with_user(&ch_config.user)
                    .with_password(&ch_config.password)
                    .with_database(&ch_config.database);
                info!("Established ClickHouse connection");
                Ok(Some(Arc::new(client)))
            } else {
                warn!("ClickHouse not configured");
                Ok(None)
            }
        });

        // Task D: Governance Settings (DB query)
        let db_pool_for_gov = db_pool.clone();
        let gov_fut: tokio::task::JoinHandle<Result<i32, anyhow::Error>> = tokio::spawn(async move {
            info!("Loading scenario governance settings from database...");
            let val: i32 = sqlx::query_scalar("SELECT (value->>0)::int FROM system_settings WHERE key = 'max_active_scenarios'")
                .fetch_one(&db_pool_for_gov)
                .await
                .unwrap_or(20);
            Ok(val)
        });

        // Wait for foundational subsystems
        let (sec_res, cache_res, ch_res, gov_res) = tokio::join!(
            security_manager_fut, 
            cache_manager_fut, 
            clickhouse_fut, 
            gov_fut
        );

        let security_manager = Arc::new(sec_res.context("Security join error")??);
        let cache_manager = Arc::new(cache_res.context("Cache join error")??);
        let clickhouse = ch_res.context("ClickHouse join error")??;
        let max_scenarios_val = gov_res.context("Governance join error")??;

        // 3. Post-foundation components
        let hot_registry = Arc::new(HotRegistry::new());
        let performance_stats = Arc::new(PerformanceStats::new());
        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));
        
        let cache_config = CacheConfig::default();
        let staging_manager = Arc::new(StagingManager::new(&config.redis.url, resilient_pool.clone(), cache_config.clone(), resilience_metrics.clone()).await?);
        let staleness_engine = Arc::new(StalenessEngine::new(staging_manager.clone(), item_feature_service.clone()));

        // 4. Concurrently load models (Heavy Task)
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_loader = Arc::new(ModelLoader::new(
            config.ml.model_path.to_str().unwrap_or("models"),
            model_repo.clone(),
            config.ml.clone(),
            resilience_metrics.clone(),
            None,
        ));

        let model_loader_clone = model_loader.clone();
        let model_load_fut: tokio::task::JoinHandle<Result<usize, anyhow::Error>> = tokio::spawn(async move {
            info!("Starting background model loading...");
            model_loader_clone.load_all_models().await.map_err(|e| e.into())
        });

        // 5. Build remaining engine components
        let scenario_factory = Arc::new(ScenarioFactory::new(resilient_pool.clone(), resilience_metrics.clone()));
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            resilience_metrics.clone(),
            Some(performance_stats.clone()),
        ));

        let strategy_resolver = Arc::new(StrategyResolver::new());

        let ingestion_metrics = Arc::new(IngestionMetrics::new(Vec::new()));
        let ingestion_manager = IngestionManager::new(
            config.ingestion.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
            staleness_engine.clone(),
            circuit_breaker_registry.clone(),
            ingestion_metrics.clone(),
            clickhouse.clone(),
        );

        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let feature_store = Arc::new(crate::ml::feature_store::FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            config.ml.clone(),
            None,
        ));

        let training_orchestrator = Arc::new(crate::ml::TrainingOrchestrator::new(
            config.ml.clone(),
            clickhouse.as_ref().map(|c| (**c).clone()).unwrap_or_else(|| clickhouse::Client::default()),
            security_manager.clone(),
            model_loader.clone(),
        ));

        let experiment_coordinator = Arc::new(ExperimentCoordinator::new(config.experiments.clone()));
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let max_active_scenarios = Arc::new(AtomicUsize::new(max_scenarios_val as usize));

        let engine = Arc::new(Self {
            config: config.clone(),
            scenarios: Arc::new(RwLock::new(HashMap::new())),
            linked_scenarios: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
            scenario_factory,
            pipeline_executor,
            strategy_resolver,
            staging_manager,
            staleness_engine,
            hot_registry: hot_registry.clone(),
            model_loader,
            training_orchestrator,
            item_feature_service,
            feature_store,
            feature_repo,
            cache_repo,
            clickhouse,
            ingestion_manager: Arc::new(RwLock::new(ingestion_manager)),
            ingestion_metrics,
            experiment_coordinator,
            max_active_scenarios,
            security_manager,
            circuit_breaker_registry,
            cache_manager,
            metrics_collector,
            performance_stats,
            shutdown_tx,
        });

        // Wait for models to finish loading
        let _ = model_load_fut.await.context("Model load join error")??;

        info!("BongasEngine ready (Concurrent startup successful)");

        let engine_clone = engine.clone();
        tokio::spawn(async move {
            info!("Starting Hot Registry pulse worker...");
            engine_clone.start_hot_registry_pulse().await;
        });

        Ok(engine)
    }

    async fn start_hot_registry_pulse(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.item_feature_service.get_popular_content(100, 10000).await {
                        Ok(items) => {
                            let hot_items: Vec<HotItem> = items.into_iter().map(|item| HotItem {
                                item_id: item.item_id,
                                score: item.trending_score,
                                metadata: serde_json::json!({
                                    "title": item.title,
                                    "view_count": item.view_count,
                                    "completion_rate": item.completion_rate,
                                    "is_explicit": item.is_explicit,
                                }),
                            }).collect();
                            
                            self.hot_registry.refresh(hot_items);
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to refresh Hot Registry");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Hot Registry pulse worker shutting down...");
                    break;
                }
            }
        }
    }

    /// Start activity ingestion from all configured sources
    pub async fn start_ingestion(
        self: &Arc<Self>,
        config: &crate::config::IngestionConfig,
    ) -> Result<()> {
        info!(
            kafka_enabled = config.kafka.enabled,
            api_enabled = config.api.enabled,
            clickhouse_enabled = config.clickhouse.enabled,
            "Starting activity ingestion..."
        );

        let mut manager = self.ingestion_manager.write().await;
        manager.start().await?;

        info!("Activity ingestion started successfully");
        Ok(())
    }

    /// Shutdown ingestion gracefully
    pub async fn shutdown_ingestion(&self) {
        info!("Shutting down activity ingestion...");
        let mut manager = self.ingestion_manager.write().await;
        manager.shutdown().await;
        info!("Activity ingestion shut down");
    }

    /// Get ingestion metrics
    pub fn ingestion_metrics(&self) -> Arc<IngestionMetrics> {
        self.ingestion_metrics.clone()
    }

    /// Get ingestion health summary
    pub async fn ingestion_health(&self) -> crate::ingestion::metrics::IngestionHealth {
        self.ingestion_metrics.health().await
    }


    /// HOT-RELOAD: Reload all scenarios from database without restart
    pub async fn reload_scenarios(&self) -> Result<usize> {
        info!("Reloading scenarios from database...");

        let mut new_scenarios = self.scenario_factory.load_all_from_db().await?;
        let max_cap = self.max_active_scenarios.load(Ordering::SeqCst);

        // Enforce Cap: If more scenarios are enabled than the limit, prioritize by priority field
        if new_scenarios.len() > max_cap {
            warn!(
                count = new_scenarios.len(),
                limit = max_cap,
                "Maximum active scenarios exceeded! Pruning based on priority..."
            );
            
            let mut sorted_scenarios: Vec<_> = new_scenarios.values().collect();
            // Higher priority first
            sorted_scenarios.sort_by(|a, b| b.pipeline.stages.len().cmp(&a.pipeline.stages.len())); // Simplified priority for now
            
            let keys_to_remove: Vec<String> = new_scenarios.keys()
                .filter(|k| !sorted_scenarios.iter().take(max_cap).any(|s| &s.slug == *k))
                .cloned()
                .collect();

            for key in keys_to_remove {
                new_scenarios.remove(&key);
            }
        }

        let mut linked_map = HashMap::new();

        for scenario in new_scenarios.values_mut() {
            match self.pipeline_executor.link(&scenario.pipeline) {
                Ok(executable) => {
                    let arc_executable = Arc::new(executable);
                    scenario.linked_pipeline = Some(arc_executable.clone());
                    linked_map.insert(scenario.slug.clone(), arc_executable);
                }
                Err(e) => {
                    warn!(slug = %scenario.slug, error = %e, "Failed to link pipeline for scenario");
                }
            }
        }

        let mut scenarios = self.scenarios.write().await;
        scenarios.clear();
        scenarios.extend(new_scenarios);

        self.linked_scenarios.store(Arc::new(linked_map));

        let count = scenarios.len();

        let onnx_scenarios = scenarios.values()
            .filter(|s| s.pipeline.stages.iter().any(|stage| stage.r#type.starts_with("onnx_")))
            .count();

        info!(
            total = count,
            onnx_enabled = onnx_scenarios,
            "Scenarios reloaded and linked"
        );

        Ok(count)
    }

    /// HOT-RELOAD: Reload a single scenario from database without restart
    pub async fn reload_scenario(&self, slug: &str) -> Result<bool> {
        info!(slug = %slug, "Reloading scenario from database...");

        if let Some(mut new_scenario) = self.scenario_factory.load_one_from_db(slug).await? {
            match self.pipeline_executor.link(&new_scenario.pipeline) {
                Ok(executable) => {
                    let arc_executable = Arc::new(executable);
                    new_scenario.linked_pipeline = Some(arc_executable.clone());
                    
                    // Update linked scenarios map
                    let mut linked_map = (**self.linked_scenarios.load()).clone();
                    linked_map.insert(slug.to_string(), arc_executable);
                    self.linked_scenarios.store(Arc::new(linked_map));
                }
                Err(e) => {
                    warn!(slug = %slug, error = %e, "Failed to link pipeline for scenario during reload");
                }
            }

            let mut scenarios = self.scenarios.write().await;
            scenarios.insert(slug.to_string(), new_scenario);
            info!(slug = %slug, "Scenario reloaded and re-linked");
            Ok(true)
        } else {
            warn!(slug = %slug, "Scenario not found in database during reload");
            Ok(false)
        }
    }

    /// Remove a scenario from the active map
    pub async fn remove_scenario(&self, slug: &str) {
        let mut scenarios = self.scenarios.write().await;
        if scenarios.remove(slug).is_some() {
            info!(slug = %slug, "Scenario removed from active engine");
            
            // Update linked scenarios map
            let mut linked_map = (**self.linked_scenarios.load()).clone();
            linked_map.remove(slug);
            self.linked_scenarios.store(Arc::new(linked_map));
        }
    }

    /// Execute scenario and return recommendations
    pub async fn execute_scenario(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> Result<Vec<RecommendationItem>> {
        let (items, _) = self.execute_scenario_with_stats(scenario_slug, user_id, context_params, None).await?;
        Ok(items)
    }

    /// Execute scenario with execution stats and persona context
    pub async fn execute_scenario_with_stats_contextual(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        profile_id: Option<String>,
        maturity_rating: Option<String>,
        device_type: Option<String>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let start_time = std::time::Instant::now();

        // 1. Check for A/B Testing Experiments
        let mut experiment_overrides = HashMap::new();
        let mut _assignments = Vec::new();
        
        if self.config.experiments.enabled {
            if let Some(uid) = user_id {
                let (assigned, overrides) = self.experiment_coordinator.assign(scenario_slug, uid);
                _assignments = assigned;
                experiment_overrides = overrides;
            }
        }

        // Build Execution Context early for strategic resolution
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut context = ExecutionContext::new(
            user_id,
            self.cache_manager.clone(),
            self.model_loader.clone(),
            self.item_feature_service.clone(),
            self.feature_store.clone(),
            request_id,
        )
        .with_hot_registry(self.hot_registry.clone())
        .with_analytics(self.performance_stats.clone())
        .with_experiment_overrides(experiment_overrides)
        .with_profile_id(profile_id.clone().unwrap_or_default())
        .with_maturity_rating(maturity_rating.clone().unwrap_or_else(|| "GE".to_string()))
        .with_device_type(
            device_type.clone().or_else(|| {
                context_params.get("device_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }).unwrap_or_default()
        )
        .with_location(
            context_params.get("location")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default()
        );

        if let Some(ref ch) = self.clickhouse {
            context = context.with_clickhouse_client(ch.clone());
        }

        // 2. STRATEGIC RESOLUTION (Phase 16)
        // Try to resolve a dynamic strategy from scenario_rules first.
        let resolved_pipeline = self.strategy_resolver.resolve(scenario_slug, &context);

        // Fallback to static linked scenario if no rule matches
        let linked_pipeline = resolved_pipeline.or_else(|| {
            self.linked_scenarios.load().get(scenario_slug).cloned()
        });
        
        let scenario = {
            let scenarios = self.scenarios.read().await;
            scenarios.get(scenario_slug)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", scenario_slug))?
        };

        // 3. Enforce Scope (Phase 10)
        // Check if user's region/context matches scenario scope
        if let Some(scope_obj) = scenario.scope.as_object() {
            if let Some(allowed_regions) = scope_obj.get("regions").and_then(|v| v.as_array()) {
                let user_region = context_params.get("region").and_then(|v| v.as_str()).unwrap_or("UNKNOWN");
                if !allowed_regions.iter().any(|r| r.as_str() == Some(user_region)) {
                    warn!(scenario = %scenario_slug, user_region, "Scenario scope mismatch: Region not allowed");
                    return Err(anyhow::anyhow!("Scenario not available in your region"));
                }
            }
        }

        let uses_onnx = scenario.pipeline.stages.iter()
            .any(|stage| stage.r#type.starts_with("onnx_"));

        let mut stats = ScenarioExecutionStats {
            scenario_slug: scenario_slug.to_string(),
            uses_onnx_inference: uses_onnx,
            pipeline_stage_count: scenario.pipeline.stages.len(),
            onnx_stage_count: scenario.pipeline.stages.iter()
                .filter(|stage| stage.r#type.starts_with("onnx_"))
                .count(),
            execution_time_ms: 0,
            cached_result: false,
        };

        // Try cache first
        // Note: L2 cache key includes context_hash, which should probably include profile/maturity if they affect results
        let mut cache_params = context_params.clone();
        if let Some(ref pid) = profile_id {
            cache_params["profile_id"] = serde_json::json!(pid);
        }
        if let Some(ref mat) = maturity_rating {
            cache_params["maturity_rating"] = serde_json::json!(mat);
        }
        if let Some(ref dev) = device_type {
            cache_params["device_type"] = serde_json::json!(dev);
        }

        let context_hash = StagingManager::hash_context(&cache_params);
        if scenario.use_l2_cache {
            if let Some(cached_items) = self.staging_manager
                .get_cached(scenario_slug, user_id, &context_hash)
                .await?
            {
                stats.cached_result = true;
                stats.execution_time_ms = start_time.elapsed().as_millis() as u64;
                
                // Track cache hit in metrics
                self.metrics_collector.record_scenario_execution(
                    scenario_slug,
                    stats.execution_time_ms,
                    true
                );

                return Ok((Self::convert_to_recommendation_items(cached_items), stats));
            }
        }

        let scored_items = if let Some(linked) = linked_pipeline {
            debug!(scenario = %scenario_slug, "Using resolved strategic path");
            self.pipeline_executor.execute_linked(&linked, &context).await?
        } else {
            warn!(scenario = %scenario_slug, "Fast Path not available, falling back to dynamic execution");
            self.pipeline_executor.execute(&scenario.pipeline, &context).await?
        };

        stats.execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Record metrics for cache miss execution
        self.metrics_collector.record_scenario_execution(
            scenario_slug,
            stats.execution_time_ms,
            false
        );

        // Save to cache
        if scenario.use_l2_cache {
            self.staging_manager.save_cached(
                scenario_slug,
                user_id,
                &context_hash,
                &scored_items,
                scenario.cache_ttl_seconds,
            ).await?;
        }

        info!(
            scenario = scenario_slug,
            uses_onnx = stats.uses_onnx_inference,
            execution_time_ms = stats.execution_time_ms,
            result_count = scored_items.len(),
            "Scenario executed"
        );

        let mut final_scored_items = scored_items;
        if let Some(l) = limit {
            final_scored_items.truncate(l);
        }

        Ok((Self::convert_to_recommendation_items(final_scored_items), stats))
    }

    /// Execute scenario with execution stats
    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execute_scenario_with_stats_contextual(
            scenario_slug,
            user_id,
            None,
            None,
            None,
            context_params,
            limit,
        ).await
    }

    /// Handle user event and trigger cache invalidation
    pub async fn handle_user_event(&self, event: UserEvent) -> Result<()> {
        self.staleness_engine.on_user_event(&event).await
    }

    /// Get staleness engine reference
    pub fn staleness_engine(&self) -> Arc<StalenessEngine> {
        self.staleness_engine.clone()
    }

    /// Get ClickHouse client reference
    // pub fn clickhouse_client(&self) -> Arc<ClickHouseClient> {
    //     self.clickhouse.clone()
    // }

    /// Reload all ONNX models (hot-reload)
    pub async fn reload_models(&self) -> Result<usize> {
        info!("Hot-reloading ONNX models...");
        let count = self.model_loader.reload_all().await?;
        info!(model_count = count, "ONNX models hot-reloaded");
        Ok(count)
    }

    /// Get count of loaded ONNX models
    pub async fn model_count(&self) -> usize {
        self.model_loader.loaded_count().await
    }

    /// Get scenario count
    pub async fn scenario_count(&self) -> usize {
        self.scenarios.read().await.len()
    }

    /// List loaded scenario slugs
    pub async fn list_scenarios(&self) -> Vec<String> {
        self.scenarios.read().await.keys().cloned().collect()
    }

    /// Identify low-performing scenarios based on ClickThrough Rate (CTR) from ClickHouse.
    /// Returns the bottom 3 scenario slugs.
    pub async fn get_low_performing_scenarios(&self) -> Vec<String> {
        if let Some(ref ch) = self.clickhouse {
            info!("Querying ClickHouse for scenario performance...");
            
            // Query CTR per scenario in the last 7 days from the OLAP path
            // CTR = clicks / impressions
            // We use the new user_interactions table we are now ingesting into.
            let query = r#"
                SELECT 
                    scenario_slug,
                    countIf(interaction_type = 'click') / GREATEST(countIf(interaction_type = 'impression'), 1) as ctr
                FROM user_interactions
                WHERE created_at >= (now() - INTERVAL 7 DAY)
                  AND scenario_slug != 'unknown'
                GROUP BY scenario_slug
                HAVING countIf(interaction_type = 'impression') > 100
                ORDER BY ctr ASC
                LIMIT 3
            "#;

            match ch.query(query).fetch_all::<(String, f64)>().await {
                Ok(results) => {
                    results.into_iter().map(|(slug, _)| slug).collect()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to fetch scenario CTR from ClickHouse user_interactions table");
                    Vec::new()
                }
            }
        } else {
            warn!("ClickHouse not available for performance pruning");
            Vec::new()
        }
    }

    /// Start cache warming background task
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval: std::time::Duration) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_manager = self.staging_manager.cache_manager();
        let cache_warmer = Arc::new(CacheWarmer::new(
            cache_manager,
            scenarios_clone,
            interval,
            self.shutdown_tx.subscribe(),
        ));

        tokio::spawn(async move {
            cache_warmer.start().await;
        });

        info!(
            scenarios = ?warm_scenarios,
            interval_secs = interval.as_secs(),
            "Cache warming started"
        );
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheMetricsSnapshot {
        self.staging_manager.cache_manager().metrics()
    }

    /// Get cache hit rate
    pub fn get_cache_hit_rate(&self) -> f64 {
        self.staging_manager.get_hit_rate()
    }

    /// Get feature repository
    pub fn feature_repo(&self) -> Arc<FeatureRepository> {
        self.feature_repo.clone()
    }


    /// Get cache repository
    pub fn cache_repo(&self) -> Arc<CacheRepository> {
        self.cache_repo.clone()
    }


    /// Get security status for API endpoint
    pub async fn get_security_status(&self) -> SecurityStatus {
        SecurityStatus {
            validated: self.security_manager.is_validated().await,
            security_enabled: true,
            layers_configured: 8,
        }
    }

    /// Get circuit breaker registry for health checks and management
    pub fn circuit_breaker_registry(&self) -> Arc<CircuitBreakerRegistry> {
        self.circuit_breaker_registry.clone()
    }

    /// Get circuit breaker health summary for all registered breakers
    pub fn circuit_breaker_health(&self) -> crate::circuit_breaker::RegistryStateSummary {
        self.circuit_breaker_registry.state_summary()
    }

    /// Convert ScoredItem to RecommendationItem
    fn convert_to_recommendation_items(scored_items: Vec<ScoredItem>) -> Vec<RecommendationItem> {
        scored_items.into_iter().map(|item| RecommendationItem {
            item_id: item.item_id,
            score: item.score,
            metadata: item.metadata,
            reasoning: item.reasoning,
        }).collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: serde_json::Value,
    pub reasoning: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScenarioExecutionStats {
    pub scenario_slug: String,
    pub uses_onnx_inference: bool,
    pub pipeline_stage_count: usize,
    pub onnx_stage_count: usize,
    pub execution_time_ms: u64,
    pub cached_result: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityStatus {
    pub validated: bool,
    pub security_enabled: bool,
    pub layers_configured: u32,
}
