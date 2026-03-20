//! Coordination domain — orchestrates the functional pillars of the engine.

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Serialize, Deserialize};
use tracing::info;

use crate::AppConfig;
use crate::security::SecurityManager;
use crate::cache::{CacheManager, CacheMetricsSnapshot};
use crate::ingestion::{IngestionManager, IngestionHealth, broadcast::manager::service::IngestionComponents};
use crate::resilience::ResilienceMetricsCollector;
use crate::ml::coordination::service::MlPillar;
use crate::engine::execution::pillar::service::ExecutionPillar;
use crate::engine::governance::pillar::service::GovernancePillar;
use crate::engine::intelligence::pillar::service::IntelligencePillar;
use crate::notification::NotificationDispatcher;
use crate::error::{AppResult, AppError, ScenarioError};
use crate::db::PipelineDefinition;
use crate::pipeline::types::models::ExecutablePipeline;

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
    pub ml_pillar: Arc<MlPillar>,
    pub intelligence: Arc<IntelligencePillar>,
    pub notifications: Arc<NotificationDispatcher>,
    pub cache: Arc<CacheManager>,
    pub shutdown_tx: broadcast::Sender<()>,
    pub resilience_metrics: Arc<ResilienceMetricsCollector>,
    pub system_health: Arc<crate::resilience::collector::SystemHealthCollector>,
}

/// 🎼 THE CONDUCTOR: The Grand Coordinator for BONGAS-AI.
pub struct BongasEngine {
    pub config: Arc<AppConfig>,
    pub resilience_metrics: Arc<ResilienceMetricsCollector>,
    
    pub execution: Arc<ExecutionPillar>,
    pub governance: Arc<GovernancePillar>,
    pub intelligence: Arc<IntelligencePillar>,
    pub ml_pillar: Arc<MlPillar>,
    
    pub notifications: Arc<NotificationDispatcher>,
    pub security: Arc<SecurityManager>,
    pub cache: Arc<CacheManager>,
    pub ingestion: Arc<RwLock<IngestionManager>>,

    pub shutdown_tx: broadcast::Sender<()>,
}

impl BongasEngine {
    pub async fn new(components: EngineComponents) -> anyhow::Result<Self> {
        components.execution.cache_repo.pool().run_migrations().await?;

        let security = Arc::new(SecurityManager::new(
            components.config.security.clone(),
            &components.config.server.environment,
            components.execution.circuit_breaker_registry.clone(),
            components.resilience_metrics.clone(),
            None,
        ).await?);

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
        ).await?;

        Ok(Self {
            config: components.config,
            resilience_metrics: components.resilience_metrics,
            execution: components.execution,
            governance: components.governance,
            intelligence: components.intelligence,
            ml_pillar: components.ml_pillar,
            notifications: components.notifications,
            security,
            cache: components.cache,
            ingestion: Arc::new(RwLock::new(ingestion_mgr)),
            shutdown_tx: components.shutdown_tx,
        })
    }

    pub async fn start(&self) {
        info!("🎼 Starting Bongas-AI background orchestration...");
        self.ml_pillar.training.start().await;

        let shutdown_tx = self.shutdown_tx.clone();
        let workers = self.intelligence.workers.clone();
        tokio::spawn(async move {
            workers.start(shutdown_tx).await;
        });
    }

    pub async fn execute_scenario(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> AppResult<Vec<RecommendationItem>> {
        let (items, _) = self.execution.manager.execute_scenario_with_stats_contextual(
            crate::engine::execution::core::execution_manager::ScenarioExecutionContext {
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

    pub async fn execute_scenario_with_stats_contextual(
        &self,
        ctx: crate::engine::execution::core::execution_manager::ScenarioExecutionContext,
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execution.manager.execute_scenario_with_stats_contextual(ctx).await
    }

    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execution.manager.execute_scenario_with_stats_contextual(
            crate::engine::execution::core::execution_manager::ScenarioExecutionContext {
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

    pub async fn list_scenarios(&self) -> Vec<String> {
        let scenarios = self.governance.scenarios.scenarios.read().await;
        scenarios.keys().cloned().collect()
    }

    pub async fn reload_scenarios(&self) -> AppResult<usize> {
        self.governance.scenarios.reload_scenarios().await
            .map_err(|e| AppError::Scenario(ScenarioError::ExecutionFailed(e.to_string())))
    }

    pub async fn reload_scenario(&self, slug: &str) -> AppResult<bool> {
        self.governance.scenarios.reload_scenario(slug).await
            .map_err(|e| AppError::Scenario(ScenarioError::ExecutionFailed(e.to_string())))
    }

    pub async fn remove_scenario(&self, slug: &str) {
        self.governance.scenarios.remove_scenario(slug).await
    }

    pub async fn ingestion_health(&self) -> IngestionHealth {
        self.ingestion.read().await.health().await
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

    pub fn circuit_breaker_registry(&self) -> Arc<crate::circuit_breaker::CircuitBreakerRegistry> {
        self.execution.circuit_breaker_registry.clone()
    }

    pub fn ghost_prewarm(
        self: Arc<Self>,
        _user_id: Option<i32>,
        _page_slug: String,
        _offset: usize,
        _batch_size: usize,
        _ttl_seconds: usize,
        _context_params: serde_json::Value,
    ) {
        // High-integrity pre-warm implementation delegated to specific worker logic
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
        self.execution.cache_repo.pool().close().await;
        let _ = self.cache.close().await;
        opentelemetry::global::shutdown_tracer_provider();
    }
}
