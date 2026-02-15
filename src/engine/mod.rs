pub mod engine;
pub mod staging_manager;
pub mod staleness_engine;
pub mod context;
pub mod config;
pub mod scenario_factory;

use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use sqlx::PgPool;
use tracing::info;

use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceConfig};
use crate::db::{ResilientPool, ResilientPoolConfig};
use crate::db::models::PipelineDefinition;
use crate::pipeline::executor::PipelineExecutor;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::ScoredItem;
use crate::cache::{CacheManager, CacheConfig, CacheWarmer, CacheMetricsSnapshot};
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::ingestion::IngestionManager;
use crate::ingestion::metrics::IngestionMetrics;
use crate::ml::model_loader::ModelLoader;
use crate::db::repositories::model_repository::ModelRepository;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::config::SecurityConfig;
use crate::security::SecurityManager;

use self::staging_manager::StagingManager;
use self::staleness_engine::{StalenessEngine, UserEvent};
use self::scenario_factory::ScenarioFactory;

/// Central orchestrator for BONGAS-AI
pub struct BongasEngine {
    // Scenario management
    scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
    scenario_factory: Arc<ScenarioFactory>,

    // Pipeline execution
    pipeline_executor: Arc<PipelineExecutor>,

    // Caching & staging
    staging_manager: Arc<StagingManager>,
    staleness_engine: Arc<StalenessEngine>,

    // ML Model Management
    model_loader: Arc<ModelLoader>,

    // Repositories
    feature_repo: Arc<FeatureRepository>,
    cache_repo: Arc<CacheRepository>,

    // Ingestion
    ingestion_manager: Arc<RwLock<Option<IngestionManager>>>,
    ingestion_metrics: Arc<IngestionMetrics>,

    // Security
    security_manager: Arc<SecurityManager>,

    // Resilience
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    resilient_pool: Arc<ResilientPool>,
    resilience_metrics: Arc<ResilienceMetricsCollector>,

    // Dependencies
    db_pool: Arc<PgPool>,
    cache_manager: Arc<CacheManager>,
}

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub pipeline: PipelineDefinition,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
}

impl BongasEngine {
    /// Create new BongasEngine
    pub async fn new(
        db_pool: PgPool,
        redis_url: &str,
        model_dir: &str,
        security_config: SecurityConfig,
        cache_config: CacheConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Result<Arc<Self>> {
        info!("Initializing BongasEngine...");

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
                security_config,
                circuit_breaker_registry.clone(),
                security_observer,
                None, // Analytics wired separately when PerformanceStats is available
            )
            .context("Failed to create SecurityManager")?,
        );

        let db_pool = Arc::new(db_pool);

        // Create Netflix-grade cache manager
        let cache_manager = Arc::new(CacheManager::new(redis_url, cache_config.clone()).await?);

        // Create model repository and loader
        let model_repo = Arc::new(ModelRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let model_loader = Arc::new(ModelLoader::new(
            model_dir,
            model_repo.clone(),
            crate::config::MlConfig::default(),
            resilience_metrics.clone(),
            None,
        ));

        // Load all deployed ONNX models
        let model_count = model_loader.load_all_models().await?;
        info!(model_count = model_count, "ONNX models loaded");

        // Create staging manager with CacheManager
        let staging_manager = Arc::new(
            StagingManager::new(redis_url, (*db_pool).clone(), cache_config).await?
        );

        // Create staleness engine
        let staleness_engine = Arc::new(StalenessEngine::new(staging_manager.clone()));

        // Create analytics manager
        // let analytics = Arc::new(AnalyticsManager::new()?);

        // Create scenario factory
        let scenario_factory = Arc::new(ScenarioFactory::new(resilient_pool.clone(), resilience_metrics.clone()));

        // Create pipeline executor with Netflix resilience
        let pipeline_config = crate::config::PipelineConfig::default();
        let pipeline_observer: Arc<dyn crate::circuit_breaker::observer::ResilienceObserver> =
            resilience_metrics.clone();
        let pipeline_executor = Arc::new(PipelineExecutor::new(
            pipeline_config,
            circuit_breaker_registry.clone(),
            pipeline_observer,
            None, // Analytics wired separately per-request via ExecutionContext
        ));

        info!(
            registered_stages = pipeline_executor.stage_count(),
            "Pipeline executor ready"
        );

        // Create Ingestion metrics registry
        let ingestion_metrics = Arc::new(IngestionMetrics::new(Vec::new()));

        // Create repositories
        let feature_repo = Arc::new(FeatureRepository::new(resilient_pool.clone(), resilience_metrics.clone()));
        let cache_repo = Arc::new(CacheRepository::new((*db_pool).clone(), resilience_metrics.clone()));

        let engine = Arc::new(Self {
            scenarios: Arc::new(RwLock::new(HashMap::new())),
            scenario_factory,
            pipeline_executor,
            staging_manager,
            staleness_engine,
            model_loader,
            feature_repo,
            cache_repo,
            ingestion_manager: Arc::new(RwLock::new(None)),
            ingestion_metrics,
            security_manager,
            circuit_breaker_registry,
            resilient_pool,
            resilience_metrics,
            db_pool,
            cache_manager,
        });

        info!("BongasEngine initialized successfully");

        Ok(engine)
    }

    /// Start activity ingestion from all configured sources
    pub async fn start_ingestion(
        self: &Arc<Self>,
        config: &crate::config::IngestionConfig,
    ) -> Result<()> {
        info!("Starting activity ingestion...");

        // Convert config to IngestionConfig
        let ingestion_config = crate::ingestion::IngestionConfig {
            kafka: crate::ingestion::sources::KafkaSourceConfig {
                brokers: config.kafka.brokers.clone(),
                group_id: config.kafka.group_id.clone(),
                playback_topic: config.kafka.playback_topic.clone(),
                reaction_topic: config.kafka.reaction_topic.clone(),
                profile_topic: config.kafka.profile_topic.clone(),
                notification_topic: config.kafka.notification_topic.clone(),
            },
            clickhouse: crate::ingestion::sources::ClickHouseSourceConfig {
                url: "http://localhost:8123".to_string(), // TODO: use from config
                poll_interval_secs: config.clickhouse.poll_interval_secs,
                enabled: config.clickhouse.enabled,
            },
            api_enabled: config.api.enabled,
        };

        // Use the legacy start method that creates everything
        let manager = IngestionManager::start_legacy(
            ingestion_config,
            self.resilient_pool.clone(),
            self.db_pool.clone(),
            self.resilience_metrics.clone(),
            self.staleness_engine.clone(),
            self.circuit_breaker_registry.clone(),
        ).await?;

        *self.ingestion_manager.write().await = Some(manager);

        info!("Activity ingestion started successfully");
        Ok(())
    }

    /// Shutdown ingestion gracefully
    pub async fn shutdown_ingestion(&self) {
        if let Some(manager) = self.ingestion_manager.write().await.take() {
            info!("Shutting down activity ingestion...");
            manager.shutdown().await;
            info!("Activity ingestion shut down");
        }
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

        let new_scenarios = self.scenario_factory.load_all_from_db().await?;

        let mut scenarios = self.scenarios.write().await;
        scenarios.clear();
        scenarios.extend(new_scenarios);

        let count = scenarios.len();

        let onnx_scenarios = scenarios.values()
            .filter(|s| s.pipeline.stages.iter().any(|stage| stage.r#type.starts_with("onnx_")))
            .count();

        info!(
            total = count,
            onnx_enabled = onnx_scenarios,
            "Scenarios reloaded"
        );

        Ok(count)
    }

    /// Execute scenario and return recommendations
    pub async fn execute_scenario(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> Result<Vec<RecommendationItem>> {
        let request_id = uuid::Uuid::new_v4().to_string();

        info!(
            request_id = %request_id,
            scenario_slug = %scenario_slug,
            user_id = ?user_id,
            "Executing scenario"
        );

        // Start recommendation timer
        // let _timer = self.analytics.start_recommendation_timer(scenario_slug);

        // Load scenario definition
        let scenario = {
            let scenarios = self.scenarios.read().await;
            scenarios.get(scenario_slug)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", scenario_slug))?
        };

        // Build context hash
        let context_hash = StagingManager::hash_context(&context_params);

        // Try to get from cache
        if scenario.use_l2_cache {
            if let Some(cached_items) = self.staging_manager
                .get_cached(scenario_slug, user_id, &context_hash)
                .await?
            {
                info!(request_id = %request_id, "Returning cached recommendations");
                return Ok(Self::convert_to_recommendation_items(cached_items));
            }
        }

        // Execute pipeline
        let context = ExecutionContext::new(
            user_id,
            self.db_pool.clone(),
            self.cache_manager.clone(),
            self.model_loader.clone(),
            request_id.clone(),
        )
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

        let scored_items = self.pipeline_executor
            .execute(&scenario.pipeline, &context)
            .await
            .with_context(|| format!("Pipeline execution failed for scenario '{}'", scenario_slug))?;

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
            request_id = %request_id,
            result_count = scored_items.len(),
            "Scenario executed successfully"
        );

        Ok(Self::convert_to_recommendation_items(scored_items))
    }

    /// Execute scenario with execution stats
    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let start_time = std::time::Instant::now();

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
                return Ok((Self::convert_to_recommendation_items(cached_items), stats));
            }
        }

        // Execute pipeline
        let request_id = uuid::Uuid::new_v4().to_string();
        let context = ExecutionContext::new(
            user_id,
            self.db_pool.clone(),
            self.cache_manager.clone(),
            self.model_loader.clone(),
            request_id,
        )
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

        let scored_items = self.pipeline_executor
            .execute(&scenario.pipeline, &context)
            .await?;

        stats.execution_time_ms = start_time.elapsed().as_millis() as u64;

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

