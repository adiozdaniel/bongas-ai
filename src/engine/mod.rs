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

use crate::db::models::PipelineDefinition;
use crate::pipeline::executor::PipelineExecutor;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::ScoredItem;
use crate::analytics::ClickHouseClient;
use crate::cache::redis::RedisClient;
use crate::cache::warming::CacheWarmer;
use crate::kafka::manager::KafkaConsumerManager;
use crate::kafka::metrics::KafkaMetricsRegistry;
use crate::ml::model_loader::ModelLoader;
use crate::db::repositories::model_repository::ModelRepository;
use crate::experiments::manager::ExperimentManager;
use crate::security::manager::SecurityManager;

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

    // Experiments & Bandits
    experiment_manager: Arc<ExperimentManager>,

    // Kafka
    kafka_manager: Arc<RwLock<Option<KafkaConsumerManager>>>,
    kafka_metrics: Arc<KafkaMetricsRegistry>,

    // Security
    security_manager: Option<Arc<SecurityManager>>,

    // Dependencies
    db_pool: Arc<PgPool>,
    clickhouse: Arc<ClickHouseClient>,
    redis: Arc<RedisClient>,
}

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub name: String,
    pub pipeline: PipelineDefinition,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
}

impl BongasEngine {
    /// Create new BongasEngine
    pub async fn new(
        db_pool: PgPool,
        clickhouse: ClickHouseClient,
        redis_url: &str,
        model_dir: &str,
        security_manager: Option<Arc<SecurityManager>>,
    ) -> Result<Arc<Self>> {
        info!("Initializing BongasEngine...");

        let db_pool = Arc::new(db_pool);
        let clickhouse = Arc::new(clickhouse);
        let redis = Arc::new(RedisClient::new(redis_url).await?);

        // Create model repository and loader
        let model_repo = Arc::new(ModelRepository::new(db_pool.as_ref().clone()));
        let model_loader = Arc::new(ModelLoader::new(model_dir, model_repo.clone()));

        // Load all deployed ONNX models
        let model_count = model_loader.load_all_models().await?;
        info!(model_count = model_count, "ONNX models loaded");

        // Create staging manager (owns its own Redis connection)
        let staging_manager = Arc::new(
            StagingManager::new(redis_url, db_pool.as_ref().clone()).await?
        );

        // Create staleness engine
        let staleness_engine = Arc::new(StalenessEngine::new(staging_manager.clone()));

        // Create scenario factory
        let scenario_factory = Arc::new(ScenarioFactory::new(db_pool.clone()));

        // Create pipeline executor
        let pipeline_executor = Arc::new(PipelineExecutor::new());

        info!(
            registered_stages = pipeline_executor.stage_count(),
            "Pipeline executor ready"
        );

        // Create Kafka metrics registry
        let kafka_metrics = Arc::new(KafkaMetricsRegistry::new());

        // Create experiment manager
        let experiment_manager = Arc::new(ExperimentManager::new(db_pool.clone()));

        let engine = Arc::new(Self {
            scenarios: Arc::new(RwLock::new(HashMap::new())),
            scenario_factory,
            pipeline_executor,
            staging_manager,
            staleness_engine,
            model_loader,
            experiment_manager,
            kafka_manager: Arc::new(RwLock::new(None)),
            kafka_metrics,
            security_manager,
            db_pool,
            clickhouse,
            redis,
        });

        info!("BongasEngine initialized successfully");

        Ok(engine)
    }

    /// Start Kafka consumers
    pub async fn start_kafka_consumers(
        self: &Arc<Self>,
        kafka_brokers: &str,
    ) -> Result<()> {
        info!("Starting Kafka consumers...");

        let mut manager = KafkaConsumerManager::new();
        manager.start_all(
            kafka_brokers,
            self.db_pool.clone(),
            self.staleness_engine.clone(),
        )?;

        *self.kafka_manager.write().await = Some(manager);

        info!("Kafka consumers started successfully");
        Ok(())
    }

    /// Shutdown Kafka consumers gracefully
    pub async fn shutdown_kafka_consumers(&self) {
        if let Some(manager) = self.kafka_manager.write().await.take() {
            info!("Shutting down Kafka consumers...");
            manager.shutdown().await;
            info!("Kafka consumers shut down");
        }
    }

    /// Get Kafka metrics
    pub fn kafka_metrics(&self) -> Arc<KafkaMetricsRegistry> {
        self.kafka_metrics.clone()
    }

    /// Get Kafka health summary
    pub async fn kafka_health(&self) -> crate::kafka::metrics::KafkaHealthSummary {
        self.kafka_metrics.health_summary().await
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
            self.clickhouse.clone(),
            self.redis.clone(),
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
            self.clickhouse.clone(),
            self.redis.clone(),
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
    pub fn clickhouse_client(&self) -> Arc<ClickHouseClient> {
        self.clickhouse.clone()
    }

    /// Get model loader reference
    pub fn model_loader(&self) -> Arc<ModelLoader> {
        self.model_loader.clone()
    }

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
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval_minutes: u64) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_warmer = Arc::new(CacheWarmer::new(
            self.clone(),
            scenarios_clone,
            interval_minutes,
        ));
        
        cache_warmer.start();
        info!(
            scenarios = ?warm_scenarios,
            interval_minutes = interval_minutes,
            "Cache warming started"
        );
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> crate::cache::metrics::CacheStatsSnapshot {
        // Convert StagingStats to CacheStatsSnapshot
        let staging_stats = self.staging_manager.get_stats();
        crate::cache::metrics::CacheStatsSnapshot {
            l1_hits: staging_stats.l1_hits,
            l1_misses: staging_stats.l1_misses,
            l2_hits: staging_stats.l2_hits,
            l2_misses: staging_stats.l2_misses,
            invalidations: staging_stats.invalidations,
            warmings: 0, // StagingManager doesn't track warmings
            overall_hit_rate: staging_stats.hit_rate,
            l1_hit_rate: 0.0, // Calculate if needed
            l2_hit_rate: 0.0, // Calculate if needed
        }
    }

    /// Get cache hit rate
    pub fn get_cache_hit_rate(&self) -> f64 {
        self.staging_manager.get_hit_rate()
    }

    /// Get security status for API endpoint
    pub async fn get_security_status(&self) -> SecurityStatus {
        if let Some(ref manager) = self.security_manager {
            SecurityStatus {
                validated: manager.is_validated().await,
                security_enabled: true,
                layers_configured: 8,
            }
        } else {
            SecurityStatus {
                validated: false,
                security_enabled: false,
                layers_configured: 0,
            }
        }
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
