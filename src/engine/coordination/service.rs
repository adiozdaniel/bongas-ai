use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Serialize, Deserialize};
use futures::StreamExt;
use tracing::{info, Instrument};

use crate::AppConfig;
use crate::db::PipelineDefinition;
use crate::pipeline::types::models::ExecutablePipeline;
use crate::security::SecurityManager;
use crate::cache::CacheManager;
use crate::ingestion::{IngestionManager, IngestionHealth, broadcast::manager::service::IngestionComponents};
use crate::resilience::ResilienceMetricsCollector;
use crate::cache::CacheMetricsSnapshot;
use crate::db::repositories::feature_repository::service::FeatureRepository;
use crate::db::repositories::cache_repository::service::CacheRepository;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::governance::factory::scenario_factory::service::ScenarioFactory;
use crate::error::{AppResult, AppError, ScenarioError};

use crate::engine::execution::pillar::service::ExecutionPillar;
use crate::engine::governance::pillar::service::GovernancePillar;
use crate::engine::intelligence::pillar::service::IntelligencePillar;

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub name: String,
    pub pipeline: PipelineDefinition,
    pub maturity_rating: String,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
    pub initial_display_limit: i32,
    pub scope: serde_json::Value,
    pub linked_pipeline: Option<Arc<ExecutablePipeline>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: serde_json::Value,
    pub reasoning: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioExecutionStats {
    pub scenario_slug: String,
    pub uses_onnx_inference: bool,
    pub pipeline_stage_count: usize,
    pub onnx_stage_count: usize,
    pub execution_time_ms: u64,
    pub cached_result: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityStatus {
    pub validated: bool,
    pub security_enabled: bool,
    pub layers_configured: u32,
}

/// Components required to initialize the BongasEngine.
pub struct EngineComponents {
    pub config: Arc<AppConfig>,
    pub execution: Arc<ExecutionPillar>,
    pub governance: Arc<GovernancePillar>,
    pub ml_pillar: Arc<crate::ml::coordination::service::MlPillar>,
    pub intelligence: Arc<IntelligencePillar>,
    pub cache: Arc<CacheManager>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub resilience_metrics: Arc<ResilienceMetricsCollector>,
}

/// 🎼 THE CONDUCTOR: The Grand Coordinator for BONGAS-AI.
/// 
/// Orchestrates the three functional pillars of the engine.
pub struct BongasEngine {
    pub config: Arc<AppConfig>,
    pub resilience_metrics: Arc<ResilienceMetricsCollector>,
    
    // The Three Pillars
    pub execution: Arc<ExecutionPillar>,
    pub governance: Arc<GovernancePillar>,
    pub intelligence: Arc<IntelligencePillar>,
    
    // Foundational Shared State
    pub security: Arc<SecurityManager>,
    pub cache: Arc<CacheManager>,
    pub ingestion: Arc<RwLock<IngestionManager>>,

    // Lifecycle
    pub shutdown_tx: broadcast::Sender<()>,
}

impl BongasEngine {
    pub async fn new(
        components: EngineComponents,
    ) -> anyhow::Result<Self> {
        // Create security manager
        let security = Arc::new(SecurityManager::new(
            components.config.security.clone(),
            &components.config.server.environment,
            components.execution.circuit_breaker_registry.clone(),
            components.resilience_metrics.clone(),
            None,
        ).await.map_err(|e| anyhow::anyhow!("Security init error: {}", e))?);

        // Create ingestion manager
        let ingestion_mgr = IngestionManager::bootstrap(
            IngestionComponents {
                pool: components.execution.cache_repo.pool(),
                intelligence: components.intelligence.clone(),
                cache_manager: components.cache.clone(),
                breaker_registry: components.execution.circuit_breaker_registry.clone(),
                resilience_metrics: components.resilience_metrics.clone(),
                staleness_engine: components.intelligence.staleness.clone(),
                pages_manager: components.governance.orchestration.clone(),
                kafka_config: components.config.ingestion.kafka.clone(),
            }
        ).await.map_err(|e| anyhow::anyhow!("Ingestion bootstrap error: {}", e))?;

        Ok(Self {
            config: components.config,
            resilience_metrics: components.resilience_metrics,
            execution: components.execution,
            governance: components.governance,
            intelligence: components.intelligence,
            security,
            cache: components.cache,
            ingestion: Arc::new(RwLock::new(ingestion_mgr)),
            shutdown_tx: components.shutdown_tx,
        })
    }

    /// Start background workers and maintenance tasks.
    pub async fn start(&self) {
        info!("🎼 Starting Bongas-AI background orchestration...");

        // Start all Intelligence Workers
        let shutdown_tx = self.shutdown_tx.clone();
        let workers = self.intelligence.workers.clone();
        tokio::spawn(async move {
            workers.start(shutdown_tx).await;
        });
    }

    /// Proxy: Execute scenario and return recommendations
    pub async fn execute_scenario(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> AppResult<Vec<RecommendationItem>> {
        let (items, _) = self.execution.manager.execute_scenario_with_stats_contextual(
            crate::engine::execution::core::execution_manager::service::ScenarioExecutionContext {
                scenario_slug: scenario_slug.to_string(),
                user_id,
                profile_id: None,
                maturity_rating: None,
                device_type: None,
                context_params,
                limit: None,
                request_id: None,
            }
        ).await?;
        Ok(items)
    }

    /// Proxy: Execute scenario with execution stats and persona context
    pub async fn execute_scenario_with_stats_contextual(
        &self,
        ctx: crate::engine::execution::core::execution_manager::service::ScenarioExecutionContext,
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let profile_id = ctx.profile_id.clone();
        let res = self.execution.manager.execute_scenario_with_stats_contextual(ctx).await?;
        
        // --- INTERNAL HOOK: Record Exposure ---
        // If profile_id is present, track these items as "seen" for content fatigue
        if let Some(ref pid) = profile_id {
            if self.config.ml.fatigue_enabled && matches!(self.config.ml.fatigue_adaptor, crate::config::types::ml::ExposureSourceAdaptor::InternalHook) {
                let item_ids: Vec<i32> = res.0.iter().map(|i| i.item_id).collect();
                let pid_clone = pid.clone();
                let fatigue_sync = self.intelligence.fatigue_sync.clone();
                tokio::spawn(async move {
                    fatigue_sync.record_exposures(&pid_clone, item_ids).await;
                });
            }
        }

        Ok(res)
    }

    /// Compatibility proxy: execute_scenario_with_stats
    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execution.manager.execute_scenario_with_stats_contextual(
            crate::engine::execution::core::execution_manager::service::ScenarioExecutionContext {
                scenario_slug: scenario_slug.to_string(),
                user_id,
                profile_id: None,
                maturity_rating: None,
                device_type: None,
                context_params,
                limit,
                request_id: None,
            }
        ).await
    }

    /// Proxy: Reload all scenarios
    pub async fn reload_scenarios(&self) -> AppResult<usize> {
        let count = self.governance.scenarios.reload_scenarios().await
            .map_err(|e| AppError::Scenario(ScenarioError::ExecutionFailed(format!("Reload failed: {}", e))))?;
        
        // Also reload discovery configurations
        self.governance.reload_discovery_configs().await?;
        
        Ok(count)
    }

    /// Proxy: Reload a single scenario
    pub async fn reload_scenario(&self, slug: &str) -> AppResult<bool> {
        self.governance.scenarios.reload_scenario(slug).await
            .map_err(|e| AppError::Scenario(ScenarioError::ExecutionFailed(format!("Reload failed for {}: {}", slug, e))))
    }

    /// Proxy: Remove a scenario
    pub async fn remove_scenario(&self, slug: &str) {
        self.governance.scenarios.remove_scenario(slug).await
    }

    /// Proxy: List loaded scenario slugs
    pub async fn list_scenarios(&self) -> Vec<String> {
        let scenarios = self.governance.scenarios.scenarios.read().await;
        scenarios.keys().cloned().collect()
    }

    pub async fn ingestion_health(&self) -> IngestionHealth {
        let manager = self.ingestion.read().await;
        manager.health().await
    }

    pub fn get_hit_rate(&self) -> f64 {
        self.execution.staging_manager.get_hit_rate()
    }

    pub fn get_cache_stats(&self) -> CacheMetricsSnapshot {
        self.cache.metrics()
    }

    pub async fn reload_models(&self) -> AppResult<usize> {
        self.execution.model_loader.reload_all().await
            .map_err(|e| AppError::Model(crate::error::ModelError::LoadFailed(format!("Reload failed: {}", e))))
    }

    pub async fn model_count(&self) -> usize {
        self.execution.model_loader.loaded_count().await
    }

    pub async fn get_security_status(&self) -> SecurityStatus {
        SecurityStatus {
            validated: self.security.is_validated().await,
            security_enabled: true,
            layers_configured: 8,
        }
    }

    pub fn feature_repo(&self) -> Arc<FeatureRepository> {
        self.execution.feature_repo.clone()
    }

    pub fn cache_repo(&self) -> Arc<CacheRepository> {
        self.execution.cache_repo.clone()
    }

    pub fn scenario_factory(&self) -> Arc<ScenarioFactory> {
        self.governance.scenario_factory.clone()
    }

    pub fn circuit_breaker_registry(&self) -> Arc<CircuitBreakerRegistry> {
        self.execution.circuit_breaker_registry.clone()
    }

    pub fn staleness_engine(&self) -> Arc<StalenessEngine> {
        self.intelligence.staleness.clone()
    }

    /// 🎼 THE FINALE: Graceful engine shutdown.
    /// Closes all connection pools and flushes pending telemetry.
    pub async fn shutdown(&self) {
        tracing::info!("🎼 Symphony shutdown initiated: Finalizing all concurrent tasks...");

        // 1. Send shutdown signal to background workers
        let _ = self.shutdown_tx.send(());

        // 2. Close Database Pool (ResilientPool)
        self.execution.cache_repo.pool().close().await;

        // 3. Close Cache Tiers (Redis)
        let _ = self.cache.close().await;

        // 4. Flush OTLP traces
        opentelemetry::global::shutdown_tracer_provider();

        tracing::info!("🎼 Symphony shutdown complete. Encore!");
    }

    /// 👻 GHOST EXECUTION: Server-side look-ahead pre-warming.
    /// Anticipates the user's next scroll by executing the next batch in the background.
    pub fn ghost_prewarm(
        self: Arc<Self>,
        user_id: Option<i32>,
        page_slug: String,
        offset: usize,
        batch_size: usize,
        ttl_seconds: usize,
        context_params: serde_json::Value,
    ) {
        let engine = self.clone();
        let span = tracing::info_span!(parent: tracing::Span::current(), "ghost_prewarm", %page_slug, %offset);
        
        tokio::spawn(async move {
            let rid = format!("ghost-{}-{}", page_slug, offset);
            let cache_key = format!("ghost:user_{:?}:page_{}:offset_{}", user_id, page_slug, offset);

            // 1. Resolve the layout for the next batch
            let layout_res = engine.governance.orchestration.get_layout_contextual(
                &page_slug,
                context_params.get("device_type").and_then(|v| v.as_str()),
                context_params.get("maturity_rating").and_then(|v| v.as_str()),
                context_params.get("visitor_id").and_then(|v| v.as_str()),
            ).await;

            let composition = match layout_res {
                Ok(Some(l)) => l.composition,
                _ => return,
            };

            // Slice the next batch
            let next_batch: Vec<crate::engine::governance::orchestration::types::models::PageCompositionItem> = composition.into_iter()
                .skip(offset)
                .take(batch_size)
                .collect();

            if next_batch.is_empty() {
                return;
            }

            // 2. Execute all scenarios in the next batch concurrently
            let results: Vec<serde_json::Value> = futures::stream::iter(next_batch)
                .map(|item| {
                    let engine = engine.clone();
                    let cp = context_params.clone();
                    let pid = cp.get("profile_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let mr = cp.get("maturity_rating").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let dt = cp.get("device_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                    
                    async move {
                        let res = engine.execute_scenario_with_stats_contextual(
                            crate::engine::execution::core::execution_manager::service::ScenarioExecutionContext {
                                scenario_slug: item.slug.clone(),
                                user_id,
                                profile_id: pid,
                                maturity_rating: mr,
                                device_type: dt,
                                context_params: cp,
                                limit: Some(20),
                                request_id: None,
                            }
                        ).await;
                        res.map(|(items, _)| (item, items))
                    }
                })
                .buffer_unordered(5)
                .filter_map(|res| async {
                    match res {
                        Ok((item, items)) => Some(serde_json::json!({
                            "title": item.slug.replace('_', " "),
                            "row_type": item.row_type,
                            "row_style": item.row_style,
                            "scenario": item.slug,
                            "items": items,
                        })),
                        Err(_) => None,
                    }
                })
                .collect::<Vec<_>>()
                .await;

            // 3. Store the entire batch in the "Ghost Cache"
            if !results.is_empty() {
                let ttl = std::time::Duration::from_secs(ttl_seconds as u64);
                let pid = context_params.get("profile_id").and_then(|v| v.as_str());
                let _ = engine.cache.set_with_ttl(&cache_key, &results, ttl, "ghost_prewarm", user_id, pid).await;
                tracing::debug!(request_id = %rid, "Ghost pre-warm complete and cached in Redis");
            }
        }.instrument(span));
    }
}
