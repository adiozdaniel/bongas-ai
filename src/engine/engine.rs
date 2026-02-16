use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use arc_swap::ArcSwap;

use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceConfig};
use crate::db::{ResilientPool, ResilientPoolConfig};
use crate::db::models::PipelineDefinition;
use crate::AppConfig;
use crate::pipeline::executor::PipelineExecutor;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::{ScoredItem, ExecutablePipeline};
use crate::cache::{CacheManager, CacheConfig, CacheWarmer, CacheMetricsSnapshot, HotRegistrySafe, HotItem};
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

use super::staging_manager::StagingManager;
use super::staleness_engine::{StalenessEngine, UserEvent};
use super::scenario_factory::ScenarioFactory;
use super::config::EngineDependencies;

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

    // Caching & staging
    pub(crate) staging_manager: Arc<StagingManager>,
    pub(crate) staleness_engine: Arc<StalenessEngine>,
    pub(crate) hot_registry: Arc<HotRegistrySafe>,

    // ML Model Management
    pub(crate) model_loader: Arc<ModelLoader>,

    // Repositories & services
    pub(crate) item_feature_service: Arc<ItemFeatureService>,
    pub(crate) feature_store: Arc<FeatureStore>,
    pub(crate) feature_repo: Arc<FeatureRepository>,
    pub(crate) cache_repo: Arc<CacheRepository>,
    pub(crate) clickhouse: Option<Arc<clickhouse::Client>>,

    // Ingestion
    pub(crate) ingestion_manager: Arc<RwLock<IngestionManager>>,
    pub(crate) ingestion_metrics: Arc<IngestionMetrics>,

    // Security
    pub(crate) security_manager: Arc<SecurityManager>,

    // Resilience
    pub(crate) circuit_breaker_registry: Arc<CircuitBreakerRegistry>,

    // Dependencies
    pub(crate) cache_manager: Arc<CacheManager>,
    pub(crate) metrics_collector: Arc<MetricsCollector>,
    pub(crate) performance_stats: Arc<PerformanceStats>,
}

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub pipeline: PipelineDefinition,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
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
        }

        info!("BongasEngine bootstrapped successfully");
        Ok(engine)
    }

    /// Create new BongasEngine
    async fn new(deps: EngineDependencies) -> Result<Arc<Self>> {
        let EngineDependencies {
            config,
            db_pool,
            circuit_breaker_registry,
            metrics_collector,
        } = deps;

        info!("Initializing BongasEngine components...");

        // Create shared resilience infrastructure
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
            Arc::new(MetricsRegistry::new(ResilienceConfig::default())),
        ));
        let resilient_pool = Arc::new(ResilientPool::from_pool(
            db_pool.clone(),
            ResilientPoolConfig::default(),
            circuit_breaker_registry.clone(),
        )?);

        // Create SecurityManager with Netflix-grade resilience
        let security_observer: Arc<dyn crate::circuit_breaker::observer::ResilienceObserver> =
            resilience_metrics.clone();
        let security_manager = Arc::new(
            SecurityManager::new(
                config.security.clone(),
                circuit_breaker_registry.clone(),
                security_observer,
                None, // Analytics wired separately when PerformanceStats is available
            )
            .context("Failed to create SecurityManager")?,
        );

        // Create Netflix-grade cache manager
        let cache_config = CacheConfig::default();
        let cache_manager = Arc::new(CacheManager::new(&config.redis.url, cache_config.clone()).await?);
        let hot_registry = Arc::new(HotRegistrySafe::new());
        let performance_stats = Arc::new(PerformanceStats::new());

        // Create model repository and loader
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_loader = Arc::new(ModelLoader::new(
            config.ml.model_path.to_str().unwrap_or("models"),
            model_repo.clone(),
            config.ml.clone(),
            resilience_metrics.clone(),
            None,
        ));

        // Load all deployed ONNX models
        let model_count = model_loader.load_all_models().await?;
        info!(model_count = model_count, "ONNX models loaded");

        // Create staging manager with CacheManager
        let staging_manager = Arc::new(
            StagingManager::new(
                &config.redis.url,
                resilient_pool.clone(),
                cache_config,
                resilience_metrics.clone(),
            ).await?
        );

        // Create staleness engine
        let staleness_engine = Arc::new(StalenessEngine::new(staging_manager.clone()));

        // Create scenario factory
        let scenario_factory = Arc::new(ScenarioFactory::new(resilient_pool.clone(), resilience_metrics.clone()));

        // Create pipeline executor with Netflix resilience
        let pipeline_observer: Arc<dyn crate::circuit_breaker::observer::ResilienceObserver> =
            resilience_metrics.clone();
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            config.pipeline.clone(),
            circuit_breaker_registry.clone(),
            pipeline_observer,
            Some(performance_stats.clone()),
        ));

        info!(
            registered_stages = pipeline_executor.stage_count(),
            "Pipeline executor ready"
        );

        // Create Ingestion metrics registry
        let ingestion_metrics = Arc::new(IngestionMetrics::new(Vec::new()));

        // Create Ingestion manager
        let ingestion_manager = IngestionManager::new(
            config.ingestion.clone(),
            resilient_pool.clone(),
            resilience_metrics.clone(),
            staleness_engine.clone(),
            circuit_breaker_registry.clone(),
            ingestion_metrics.clone(),
        );

        // Create repositories & services
        let item_feature_service = Arc::new(ItemFeatureService::new(resilient_pool.clone(), resilience_metrics.clone()));
        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let feature_store = Arc::new(crate::ml::feature_store::FeatureStore::new(
            resilient_pool.clone(),
            cache_manager.clone(),
            config.ml.clone(),
            None, // Analytics wired separately when PerformanceStats is available
        ));

        // Create ClickHouse client if configured
        let clickhouse = if !config.clickhouse.url.is_empty() {
            Some(Arc::new(
                clickhouse::Client::default()
                    .with_url(&config.clickhouse.url)
                    .with_user(&config.clickhouse.user)
                    .with_password(&config.clickhouse.password)
                    .with_database(&config.clickhouse.database)
            ))
        } else {
            None
        };

        let engine = Arc::new(Self {
            config: config.clone(),
            scenarios: Arc::new(RwLock::new(HashMap::new())),
            linked_scenarios: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
            scenario_factory,
            pipeline_executor,
            staging_manager,
            staleness_engine,
            hot_registry: hot_registry.clone(),
            model_loader,
            item_feature_service,
            feature_store,
            feature_repo,
            cache_repo,
            clickhouse,
            ingestion_manager: Arc::new(RwLock::new(ingestion_manager)),
            ingestion_metrics,
            security_manager,
            circuit_breaker_registry,
            cache_manager,
            metrics_collector,
            performance_stats,
        });

        info!("BongasEngine initialized successfully");

        // Start hot registry pulse
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.start_hot_registry_pulse().await;
        });

        Ok(engine)
    }

    async fn start_hot_registry_pulse(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            
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
    }

    /// Start activity ingestion from all configured sources
    pub async fn start_ingestion(
        self: &Arc<Self>,
        _config: &crate::config::IngestionConfig,
    ) -> Result<()> {
        info!("Starting activity ingestion...");

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
        let (items, _stats) = self.execute_scenario_with_stats(scenario_slug, user_id, context_params).await?;
        Ok(items)
    }

    /// Execute scenario with execution stats
    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let start_time = std::time::Instant::now();

        // Check for linked scenario first (Fast Path)
        let linked_pipeline = self.linked_scenarios.load().get(scenario_slug).cloned();
        
        let scenario = {
            let scenarios = self.scenarios.read().await;
            scenarios.get(scenario_slug)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", scenario_slug))?
        };

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
        let context_hash = StagingManager::hash_context(&context_params);
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

        // Execute pipeline
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
        .with_device_type(
            context_params.get("device_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default()
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

        let scored_items = if let Some(linked) = linked_pipeline {
            debug!(scenario = %scenario_slug, "Using Fast Path (linked pipeline)");
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

        Ok((Self::convert_to_recommendation_items(scored_items), stats))
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

    /// Start cache warming background task
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval: std::time::Duration) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_manager = self.staging_manager.cache_manager();
        let cache_warmer = Arc::new(CacheWarmer::new(
            cache_manager,
            scenarios_clone,
            interval,
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
        }).collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: serde_json::Value,
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
