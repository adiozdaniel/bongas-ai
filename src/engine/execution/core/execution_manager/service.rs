//! High-performance recommendation execution loop.

use crate::error::{AppResult, AppError, ScenarioError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::executor::PipelineExecutor;
use crate::engine::execution::cache::staging_manager::service::StagingManager;
use crate::engine::governance::strategy::resolver::service::StrategyResolver;
use crate::engine::coordination::service::{RecommendationItem, ScenarioExecutionStats, ScenarioDefinition};
use crate::cache::CacheManager;
use crate::ml::model_loader::ModelLoader;
use crate::db::repositories::item_feature_service::ItemFeatureService;
use crate::ml::FeatureStore;
use crate::analytics::types::PerformanceStats;
use crate::experiments::ExperimentCoordinator;
use crate::middlewares::MetricsCollector;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::circuit_breaker::CircuitBreakerRegistry;
use tokio::sync::RwLock;
use arc_swap::ArcSwap;
use crate::pipeline::ExecutablePipeline;

pub struct ExecutionManager {
    pub(crate) pipeline_executor: Arc<PipelineExecutor>,
    pub(crate) strategy_resolver: Arc<StrategyResolver>,
    pub(crate) staging_manager: Arc<StagingManager>,
    pub(crate) cache_manager: Arc<CacheManager>,
    pub(crate) model_loader: Arc<ModelLoader>,
    pub(crate) item_feature_service: Arc<ItemFeatureService>,
    pub(crate) feature_store: Arc<FeatureStore>,
    pub(crate) performance_stats: Arc<PerformanceStats>,
    pub(crate) experiment_coordinator: Arc<ExperimentCoordinator>,
    pub(crate) metrics_collector: Arc<MetricsCollector>,
    pub(crate) clickhouse: Option<Arc<clickhouse::Client>>,
    pub(crate) hot_registry: Arc<crate::cache::HotRegistry>,
    pub(crate) _feature_repo: Arc<FeatureRepository>,
    pub(crate) _cache_repo: Arc<CacheRepository>,
    pub(crate) _circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    
    // Coordination with Scenarios
    pub(crate) scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
    pub(crate) linked_scenarios: Arc<ArcSwap<HashMap<String, Arc<ExecutablePipeline>>>>,

    // Fix #72: Thundering Herd protection
    pub(crate) request_consolidation: Arc<dashmap::DashMap<String, Arc<tokio::sync::broadcast::Sender<Vec<RecommendationItem>>>>>,
}

impl ExecutionManager {
    pub fn new(
        pipeline_executor: Arc<PipelineExecutor>,
        strategy_resolver: Arc<StrategyResolver>,
        staging_manager: Arc<StagingManager>,
        cache_manager: Arc<CacheManager>,
        model_loader: Arc<ModelLoader>,
        item_feature_service: Arc<ItemFeatureService>,
        feature_store: Arc<FeatureStore>,
        performance_stats: Arc<PerformanceStats>,
        experiment_coordinator: Arc<ExperimentCoordinator>,
        metrics_collector: Arc<MetricsCollector>,
        clickhouse: Option<Arc<clickhouse::Client>>,
        hot_registry: Arc<crate::cache::HotRegistry>,
        feature_repo: Arc<FeatureRepository>,
        cache_repo: Arc<CacheRepository>,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
        scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
        linked_scenarios: Arc<ArcSwap<HashMap<String, Arc<ExecutablePipeline>>>>,
    ) -> Self {
        Self {
            pipeline_executor,
            strategy_resolver,
            staging_manager,
            cache_manager,
            model_loader,
            item_feature_service,
            feature_store,
            performance_stats,
            experiment_coordinator,
            metrics_collector,
            clickhouse,
            hot_registry,
            _feature_repo: feature_repo,
            _cache_repo: cache_repo,
            _circuit_breaker_registry: circuit_breaker_registry,
            scenarios,
            linked_scenarios,
            request_consolidation: Arc::new(dashmap::DashMap::new()),
        }
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
    ) -> AppResult<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        let start_time = std::time::Instant::now();

        let mut experiment_overrides = HashMap::new();
        if let Some(uid) = user_id {
            let (_, overrides) = self.experiment_coordinator.assign(scenario_slug, uid);
            experiment_overrides = overrides;
        }

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

        let resolved_pipeline = self.strategy_resolver.resolve(scenario_slug, &context);
        let linked_pipeline = resolved_pipeline.or_else(|| {
            self.linked_scenarios.load().get(scenario_slug).cloned()
        });
        
        let scenario = {
            let scenarios: tokio::sync::RwLockReadGuard<'_, HashMap<String, ScenarioDefinition>> = self.scenarios.read().await;
            scenarios.get(scenario_slug)
                .cloned()
                .ok_or_else(|| AppError::Scenario(ScenarioError::NotFound(scenario_slug.to_string())))?
        };

        if let Some(scope_obj) = scenario.scope.as_object() {
            if let Some(allowed_regions) = scope_obj.get("regions").and_then(|v| v.as_array()) {
                let user_region = context_params.get("region").and_then(|v| v.as_str()).unwrap_or("UNKNOWN");
                if !allowed_regions.iter().any(|r: &serde_json::Value| r.as_str() == Some(user_region)) {
                    warn!(scenario = %scenario_slug, user_region, "Scenario scope mismatch: Region not allowed");
                    return Err(AppError::Scenario(ScenarioError::InvalidConfig(format!("Scenario not available in region: {}", user_region))));
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

        let mut cache_params = context_params.clone();
        if let Some(ref pid) = profile_id { cache_params["profile_id"] = serde_json::json!(pid); }
        if let Some(ref mat) = maturity_rating { cache_params["maturity_rating"] = serde_json::json!(mat); }
        if let Some(ref dev) = device_type { cache_params["device_type"] = serde_json::json!(dev); }

        let context_hash = StagingManager::hash_context(&cache_params);

        let consolidation_key = format!("{}:{}:{}", scenario_slug, user_id.unwrap_or(0), context_hash);
        let waiter = {
            let entry = self.request_consolidation.entry(consolidation_key.clone());
            match entry {
                dashmap::mapref::entry::Entry::Occupied(ref e) => {
                    let tx: Arc<tokio::sync::broadcast::Sender<Vec<RecommendationItem>>> = e.get().clone();
                    Some(tx.subscribe())
                }
                dashmap::mapref::entry::Entry::Vacant(e) => {
                    let (tx, _) = tokio::sync::broadcast::channel::<Vec<RecommendationItem>>(1);
                    e.insert(Arc::new(tx));
                    None
                }
            }
        };

        if let Some(mut rx) = waiter {
            match rx.recv().await {
                Ok(items) => {
                    stats.execution_time_ms = start_time.elapsed().as_millis() as u64;
                    stats.cached_result = true;
                    return Ok((items, stats));
                }
                Err(_) => {
                    warn!(scenario = %scenario_slug, "Consolidation waiter failed, falling back");
                }
            }
        }

        if scenario.use_l2_cache {
            if let Some(cached_items) = self.staging_manager
                .get_cached(scenario_slug, user_id, &context_hash)
                .await?
            {
                stats.cached_result = true;
                stats.execution_time_ms = start_time.elapsed().as_millis() as u64;
                self.metrics_collector.record_scenario_execution(scenario_slug, stats.execution_time_ms, true).await;
                return Ok((self.convert_to_recommendation_items(cached_items), stats));
            }
        }

        let scored_items = if let Some(linked) = linked_pipeline {
            self.pipeline_executor.execute_linked(&linked, &context).await?
        } else {
            self.pipeline_executor.execute(&scenario.pipeline, &context).await?
        };

        stats.execution_time_ms = start_time.elapsed().as_millis() as u64;

        let final_recommendations = self.convert_to_recommendation_items(scored_items.clone());
        if let Some((_, tx)) = self.request_consolidation.remove(&consolidation_key) {
            let _ = tx.send(final_recommendations.clone());
        }

        self.metrics_collector.record_scenario_execution(scenario_slug, stats.execution_time_ms, false).await;

        if scenario.use_l2_cache {
            self.staging_manager.save_cached(scenario_slug, user_id, &context_hash, &scored_items, scenario.cache_ttl_seconds).await?;
        }

        let mut final_scored_items = scored_items;
        if let Some(l) = limit { final_scored_items.truncate(l); }

        Ok((self.convert_to_recommendation_items(final_scored_items), stats))
    }

    fn convert_to_recommendation_items(&self, scored_items: Vec<crate::pipeline::ScoredItem>) -> Vec<RecommendationItem> {
        scored_items.into_iter().map(|item| RecommendationItem {
            item_id: item.item_id,
            score: item.score,
            metadata: item.metadata,
            reasoning: item.reasoning,
        }).collect()
    }
}
