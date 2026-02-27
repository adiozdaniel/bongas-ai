use std::sync::Arc;
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use crate::AppConfig;
use crate::db::models::PipelineDefinition;
use crate::pipeline::ExecutablePipeline;
use crate::engine::scenarios_manager::ScenariosManager;
use crate::engine::execution_manager::ExecutionManager;
use crate::engine::staging_manager::StagingManager;
use crate::engine::staleness_engine::StalenessEngine;
use crate::engine::analytics_sidecar::AnalyticsSidecar;
use crate::engine::hive_mind::HiveMindConnector;
use crate::security::SecurityManager;
use crate::cache::CacheManager;
use crate::ingestion::IngestionManager;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::repositories::page_layout_repository::PageLayoutRepository;

#[derive(Debug, Clone)]
pub struct ScenarioDefinition {
    pub slug: String,
    pub name: String,
    pub pipeline: PipelineDefinition,
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

/// The Grand Coordinator for BONGAS-AI.
pub struct BongasEngine {
    pub config: Arc<AppConfig>,
    pub resilience_metrics: Arc<ResilienceMetricsCollector>,
    
    // Domain Managers
    pub scenarios: Arc<ScenariosManager>,
    pub execution: Arc<ExecutionManager>,
    
    // Foundational Components
    pub staging_manager: Arc<StagingManager>,
    pub staleness_engine: Arc<StalenessEngine>,
    pub security_manager: Arc<SecurityManager>,
    pub cache_manager: Arc<CacheManager>,
    pub ingestion_manager: Arc<RwLock<IngestionManager>>,
    pub analytics_sidecar: Arc<AnalyticsSidecar>,
    pub hive_mind_connector: Arc<HiveMindConnector>,
    pub page_layout_repo: Arc<PageLayoutRepository>,

    // Lifecycle
    pub shutdown_tx: broadcast::Sender<()>,
}
