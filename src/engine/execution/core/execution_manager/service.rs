//! Execution Manager Service.
//! Orchestrates the execution of recommendation pipelines.

use std::sync::Arc;
use tokio::sync::RwLock;
use arc_swap::ArcSwap;
use std::collections::HashMap;
use tracing::{warn, error};
use std::time::Instant;

use crate::pipeline::executor::service::PipelineExecutor;
use crate::engine::governance::strategy::resolver::service::StrategyResolver;
use crate::engine::execution::cache::staging_manager::service::StagingManager;
use crate::cache::manager::service::CacheManager;
use crate::ml::assets::loader::service::ModelLoader;
use crate::db::ItemFeatureService;
use crate::ml::inference::features::service::FeatureStore;
use crate::search::EmbeddedSearchManager;
use crate::pipeline::types::models::ExecutablePipeline;
use crate::engine::coordination::service::{RecommendationItem, ScenarioExecutionStats, ScenarioDefinition};
use crate::error::{AppResult, AppError, ScenarioError};
use crate::pipeline::context::service::ExecutionContext;

/// Contextual execution structure for a single scenario request.
pub struct ScenarioExecutionContext {
    pub scenario_slug: String,
    pub user_id: Option<i32>,
    pub profile_id: Option<String>,
    pub maturity_rating: Option<String>,
    pub device_type: Option<String>,
    pub context_params: serde_json::Value,
    pub limit: Option<usize>,
    pub request_id: Option<String>,
}

/// Components required to initialize the ExecutionManager.
pub struct ExecutionManagerComponents {
    pub pipeline_executor: Arc<PipelineExecutor>,
    pub strategy_resolver: Arc<StrategyResolver>,
    pub staging_manager: Arc<StagingManager>,
    pub cache_manager: Arc<CacheManager>,
    pub model_loader: Arc<ModelLoader>,
    pub item_feature_service: Arc<ItemFeatureService>,
    pub feature_store: Arc<FeatureStore>,
    pub clickhouse: Option<Arc<clickhouse::Client>>,
    pub search_manager: Option<Arc<EmbeddedSearchManager>>,
    pub hot_registry: Arc<crate::cache::HotRegistry>,
    pub scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
    pub linked_scenarios: Arc<ArcSwap<HashMap<String, Arc<ExecutablePipeline>>>>,
}

/// Orchestrates the high-integrity execution of recommendation pipelines.
pub struct ExecutionManager {
    pub(crate) pipeline_executor: Arc<PipelineExecutor>,
    pub(crate) strategy_resolver: Arc<StrategyResolver>,
    pub(crate) staging_manager: Arc<StagingManager>,
    pub(crate) cache_manager: Arc<CacheManager>,
    pub(crate) model_loader: Arc<ModelLoader>,
    pub(crate) item_feature_service: Arc<ItemFeatureService>,
    pub(crate) feature_store: Arc<FeatureStore>,
    pub(crate) clickhouse: Option<Arc<clickhouse::Client>>,
    pub(crate) search_manager: Option<Arc<EmbeddedSearchManager>>,
    pub(crate) hot_registry: Arc<crate::cache::HotRegistry>,
    pub(crate) scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
}

impl ExecutionManager {
    /// Create new ExecutionManager using high-integrity component injection.
    pub fn new(components: ExecutionManagerComponents) -> Self {
        Self {
            pipeline_executor: components.pipeline_executor,
            strategy_resolver: components.strategy_resolver,
            staging_manager: components.staging_manager,
            cache_manager: components.cache_manager,
            model_loader: components.model_loader,
            item_feature_service: components.item_feature_service,
            feature_store: components.feature_store,
            clickhouse: components.clickhouse,
            search_manager: components.search_manager,
            hot_registry: components.hot_registry,
            scenarios: components.scenarios,
        }
    }

    /// Execute scenario with stats and contextual routing.
    pub async fn execute_scenario_with_stats_contextual(
        &self,
        ctx: ScenarioExecutionContext,
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let start = Instant::now();
        let rid = ctx.request_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 1. Resolve Scenario and Pipeline
        let mut context = ExecutionContext::new(
            ctx.user_id,
            self.cache_manager.clone(),
            self.model_loader.clone(),
            self.item_feature_service.clone(),
            self.feature_store.clone(),
            rid.clone(),
        )
        .with_context_params(ctx.context_params.clone());

        context.profile_id = ctx.profile_id.clone();

        if let Some(ref ch) = self.clickhouse {
            context = context.with_clickhouse_client(ch.clone());
        }

        if let Some(ref sm) = self.search_manager {
            context = context.with_search_manager(sm.clone());
        }

        let scenario = {
            let scenarios: tokio::sync::RwLockReadGuard<'_, HashMap<String, ScenarioDefinition>> = self.scenarios.read().await;
            scenarios.get(&ctx.scenario_slug)
                .cloned()
                .ok_or_else(|| AppError::Scenario(ScenarioError::NotFound(ctx.scenario_slug.clone())))?
        };

        if let Some(scope_obj) = scenario.scope.as_object() {
            if let Some(allowed_regions) = scope_obj.get("regions").and_then(|v| v.as_array()) {
                let user_region = ctx.context_params.get("region")
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .unwrap_or("UNKNOWN");
                
                if !allowed_regions.iter().any(|r: &serde_json::Value| r.as_str() == Some(user_region)) {
                    warn!(scenario = %ctx.scenario_slug, user_region, "Scenario scope mismatch: Region not allowed");
                    return Err(AppError::Scenario(ScenarioError::InvalidConfig(format!("Scenario not available in region: {}", user_region))));
                }
            }
        }

        let uses_onnx = scenario.pipeline.stages.iter()
            .any(|stage| stage.r#type.starts_with("onnx_"));

        let mut stats = ScenarioExecutionStats {
            scenario_slug: ctx.scenario_slug.clone(),
            uses_onnx_inference: uses_onnx,
            pipeline_stage_count: scenario.pipeline.stages.len(),
            onnx_stage_count: scenario.pipeline.stages.iter()
                .filter(|stage| stage.r#type.starts_with("onnx_"))
                .count(),
            execution_time_ms: 0,
            cached_result: false,
        };

        let items_res = self.pipeline_executor.execute(&scenario.pipeline, &context).await;
        
        match items_res {
            Ok(items) => {
                let rec_items: Vec<RecommendationItem> = items.into_iter()
                    .map(|i| RecommendationItem {
                        item_id: i.item_id,
                        score: i.score,
                        metadata: i.metadata,
                        reasoning: i.reasoning,
                    })
                    .collect();
                
                stats.execution_time_ms = start.elapsed().as_millis() as u64;
                Ok((rec_items, stats))
            }
            Err(e) => {
                let err_msg = format!("Pipeline execution failed: {:?}", e);
                error!(error = %err_msg, scenario = %ctx.scenario_slug, "Execution failed");
                Err(AppError::Scenario(ScenarioError::ExecutionFailed(err_msg)))
            }
        }
    }
}
