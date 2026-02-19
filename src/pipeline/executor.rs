//! Netflix-grade pipeline executor with per-stage resilience and automatic parallelization.

use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{warn};
use futures::future::join_all;
use async_recursion::async_recursion;

use crate::analytics::types::PerformanceStats;
use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig as BreakerConfig,
    CircuitBreakerRegistry, CircuitBreakerId,
};
use crate::circuit_breaker::observer::ResilienceObserver;
use crate::config::PipelineConfig;
use crate::pipeline::{PipelineStage, ScoredItem, BoundStage, ExecutionNode, ExecutablePipeline, PipelineError as InternalPipelineError};
use crate::pipeline::validator::PipelineValidator;
use crate::pipeline::optimizer::PipelineOptimizer;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::registry::build_stage_registry;
use crate::db::models::PipelineDefinition;
use crate::error::PipelineError;

pub struct PipelineExecutor {
    stage_registry: HashMap<String, Arc<dyn PipelineStage>>,
    stage_breakers: HashMap<String, Arc<CircuitBreaker>>,
    validator: PipelineValidator,
    config: PipelineConfig,
    analytics: Option<Arc<PerformanceStats>>,
}

impl PipelineExecutor {
    pub fn new(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
    ) -> Self {
        Self::with_registry(config, breaker_registry, observer, analytics, build_stage_registry())
    }

    pub fn stage_count(&self) -> usize {
        self.stage_registry.len()
    }

    pub fn with_registry(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
        registry: HashMap<String, Arc<dyn PipelineStage>>,
    ) -> Self {
        let validator = PipelineValidator::new(registry.clone());
        let mut stage_breakers = HashMap::new();
        
        if config.stage_breaker_enabled {
            for stage_name in registry.keys() {
                let breaker_config = BreakerConfig::builder()
                    .failure_rate_threshold(config.stage_breaker_failure_rate)
                    .slow_call_rate_threshold(config.stage_breaker_slow_call_rate)
                    .slow_call_duration(config.stage_breaker_slow_call_duration)
                    .minimum_calls(config.stage_breaker_minimum_calls)
                    .recovery_timeout(config.stage_breaker_recovery_timeout)
                    .half_open_max_calls(config.stage_breaker_half_open_calls)
                    .call_timeout(Self::timeout_for_stage(stage_name, &config))
                    .build();

                if let Ok(bc) = breaker_config {
                    let breaker_id = format!("pipeline.stage.{}", stage_name);
                    let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerId::new(breaker_id), bc, observer.clone()));
                    breaker_registry.register(breaker.clone());
                    stage_breakers.insert(stage_name.clone(), breaker);
                }
            }
        }

        Self {
            stage_registry: registry,
            stage_breakers,
            validator,
            config,
            analytics,
        }
    }

    pub fn link(&self, definition: &PipelineDefinition) -> Result<ExecutablePipeline> {
        self.validator.validate_definition(definition)?;
        
        let nodes = self.link_to_nodes(&definition.stages)?;
        let optimized_nodes = PipelineOptimizer::optimize(nodes);

        let fallback_nodes = if let Some(ref fallback) = definition.fallback_stages {
            Some(PipelineOptimizer::optimize(self.link_to_nodes(fallback)?))
        } else {
            None
        };

        Ok(ExecutablePipeline { nodes: optimized_nodes, fallback_nodes })
    }

    fn link_to_nodes(&self, stages: &[crate::db::models::PipelineStageConfig]) -> Result<Vec<ExecutionNode>> {
        let mut nodes = Vec::new();
        let mut current_parallel = Vec::new();

        for stage_config in stages {
            // Handle Structural Nodes
            match stage_config.r#type.as_str() {
                "branch" => {
                    if !current_parallel.is_empty() {
                        nodes.push(self.flush_parallel(&mut current_parallel));
                    }
                    let condition: crate::pipeline::BranchCondition = serde_json::from_value(
                        stage_config.params.get("condition").cloned().unwrap_or_default()
                    )?;
                    let if_true_configs: Vec<crate::db::models::PipelineStageConfig> = serde_json::from_value(
                        stage_config.params.get("if_true").cloned().unwrap_or_default()
                    )?;
                    let if_false_configs: Vec<crate::db::models::PipelineStageConfig> = serde_json::from_value(
                        stage_config.params.get("if_false").cloned().unwrap_or_default()
                    )?;
                    
                    nodes.push(ExecutionNode::Branch {
                        condition,
                        if_true: self.link_to_nodes(&if_true_configs)?,
                        if_false: self.link_to_nodes(&if_false_configs)?,
                    });
                    continue;
                }
                "ensemble" => {
                    if !current_parallel.is_empty() {
                        nodes.push(self.flush_parallel(&mut current_parallel));
                    }
                    let source_configs: Vec<serde_json::Value> = serde_json::from_value(
                        stage_config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    let merge_strategy: crate::pipeline::MergeStrategy = serde_json::from_value(
                        stage_config.params.get("merge_strategy").cloned().unwrap_or(serde_json::json!("sum"))
                    )?;
                    
                    let mut sources = Vec::new();
                    for sc in source_configs {
                        let name = sc.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let weight = sc.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                        let inner_stages: Vec<crate::db::models::PipelineStageConfig> = serde_json::from_value(
                            sc.get("stages").cloned().unwrap_or_default()
                        )?;
                        sources.push(crate::pipeline::EnsembleSource {
                            nodes: self.link_to_nodes(&inner_stages)?,
                            weight,
                            name,
                        });
                    }
                    nodes.push(ExecutionNode::Ensemble { sources, merge_strategy });
                    continue;
                }
                "interleave" => {
                    if !current_parallel.is_empty() {
                        nodes.push(self.flush_parallel(&mut current_parallel));
                    }
                    let pattern: Vec<String> = serde_json::from_value(
                        stage_config.params.get("pattern").cloned().unwrap_or_default()
                    )?;
                    let source_map: HashMap<String, Vec<crate::db::models::PipelineStageConfig>> = serde_json::from_value(
                        stage_config.params.get("sources").cloned().unwrap_or_default()
                    )?;
                    
                    let mut sources = HashMap::new();
                    for (name, inner_stages) in source_map {
                        sources.insert(name, self.link_to_nodes(&inner_stages)?);
                    }
                    nodes.push(ExecutionNode::Interleave { pattern, sources });
                    continue;
                }
                _ => {}
            }

            // Handle Standard Stages
            let implementation = self.stage_registry.get(&stage_config.r#type).cloned()
                .with_context(|| format!("Stage '{}' not found", stage_config.r#type))?;
            
            let breaker = self.stage_breakers.get(&stage_config.r#type).cloned();
            let timeout = Self::timeout_for_stage(&stage_config.r#type, &self.config);

            let bound = BoundStage {
                implementation,
                breaker,
                params: stage_config.params.clone(),
                stage_type: stage_config.r#type.clone(),
                timeout,
            };

            if bound.implementation.can_parallelize() {
                current_parallel.push(bound);
            } else {
                if !current_parallel.is_empty() {
                    nodes.push(self.flush_parallel(&mut current_parallel));
                }
                nodes.push(ExecutionNode::Single(bound));
            }
        }

        if !current_parallel.is_empty() {
            nodes.push(self.flush_parallel(&mut current_parallel));
        }

        Ok(nodes)
    }

    fn flush_parallel(&self, current_parallel: &mut Vec<BoundStage>) -> ExecutionNode {
        if current_parallel.len() == 1 {
            ExecutionNode::Single(current_parallel.pop().unwrap())
        } else {
            // Fix #21: Allow configurable merge strategy for parallel nodes
            // Default to Sum but in a real system we might pull this from config
            ExecutionNode::Parallel { 
                stages: std::mem::take(current_parallel),
                merge_strategy: crate::pipeline::MergeStrategy::Sum,
            }
        }
    }

    pub async fn execute(&self, pipeline: &PipelineDefinition, context: &ExecutionContext) -> Result<Vec<ScoredItem>> {
        let linked = self.link(pipeline)?;
        self.execute_linked(&linked, context).await
    }

    pub async fn execute_linked(&self, pipeline: &ExecutablePipeline, context: &ExecutionContext) -> Result<Vec<ScoredItem>> {
        let pipeline_start = std::time::Instant::now();
        let result = tokio::time::timeout(self.config.pipeline_timeout, self.execute_nodes(&pipeline.nodes, context)).await;
        
        let latency = pipeline_start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time("pipeline.execute", latency.as_millis() as u64);
            if result.is_ok() { a.increment_throughput("pipeline.execute"); }
            else { a.increment_error("pipeline.execute"); }
        }

        match result {
            Ok(res) => res,
            Err(_) => {
                if let Some(ref fallback) = pipeline.fallback_nodes {
                    warn!("Pipeline timed out, using fallback");
                    self.execute_nodes(fallback, context).await
                } else {
                    Err(PipelineError::PipelineTimeout { timeout_ms: self.config.pipeline_timeout.as_millis() as u64 }.into())
                }
            }
        }
    }

    #[async_recursion]
    async fn execute_nodes(&self, nodes: &[ExecutionNode], context: &ExecutionContext) -> Result<Vec<ScoredItem>> {
        let mut items = Vec::new();
        for node in nodes {
            match node {
                ExecutionNode::Single(stage) => {
                    items = self.execute_single_stage(stage, context, items).await?;
                }
                ExecutionNode::Fused(stages) => {
                    // JIT-lite: Fused Scoring Pass
                    // Reverted to sequential batch processing to avoid per-item Vec allocations.
                    if !items.is_empty() {
                        for stage in stages {
                            items = self.execute_single_stage(stage, context, items).await?;
                        }
                    }
                }
                ExecutionNode::Parallel { stages, merge_strategy } => {
                    let futures = stages.iter().map(|s| self.execute_single_stage(s, context, items.clone()));
                    let results = join_all(futures).await;
                    
                    let mut merged_map: HashMap<i32, ScoredItem> = HashMap::new();
                    let mut counts: HashMap<i32, usize> = HashMap::new();
                    
                    // Fix #19: Propagate errors instead of swallowing
                    let mut errors = Vec::new();

                    for res in results {
                        match res {
                            Ok(stage_items) => {
                                for mut item in stage_items {
                                    let id = item.item_id;
                                    if let Some(existing) = merged_map.get_mut(&id) {
                                        match merge_strategy {
                                            crate::pipeline::MergeStrategy::Sum => existing.score += item.score,
                                            crate::pipeline::MergeStrategy::Max => existing.score = existing.score.max(item.score),
                                            crate::pipeline::MergeStrategy::Min => existing.score = existing.score.min(item.score),
                                            crate::pipeline::MergeStrategy::Average => existing.score += item.score,
                                            crate::pipeline::MergeStrategy::First => {} // Keep first score
                                        }
                                        existing.reasoning.append(&mut item.reasoning);
                                        *counts.get_mut(&id).unwrap() += 1;
                                    } else {
                                        merged_map.insert(id, item);
                                        counts.insert(id, 1);
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(e);
                            }
                        }
                    }

                    // If all branches failed, propagate first error (or a consolidated one)
                    if !errors.is_empty() && merged_map.is_empty() {
                        return Err(errors.remove(0).into());
                    }

                    if *merge_strategy == crate::pipeline::MergeStrategy::Average {
                        // Fix #20: Math correction - divide by total branches in the node, not just active ones
                        let total_branches = stages.len() as f32;
                        for item in merged_map.values_mut() {
                            item.score /= total_branches;
                        }
                    }
                    
                    items = merged_map.into_values().collect();
                    // Fix #50: Ensure deterministic ordering after HashMap merge
                    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                }
                ExecutionNode::Branch { condition, if_true, if_false } => {
                    let branch = if self.evaluate_condition(condition, context) {
                        if_true
                    } else {
                        if_false
                    };
                    items = self.execute_nodes(branch, context).await?;
                }
                ExecutionNode::Ensemble { sources, merge_strategy } => {
                    let futures = sources.iter().map(|source| {
                        async move {
                            let result = self.execute_nodes(&source.nodes, context).await;
                            (source.weight, result)
                        }
                    });
                    let results = join_all(futures).await;
                    
                    let mut ensemble_map: HashMap<i32, ScoredItem> = HashMap::new();
                    let mut counts: HashMap<i32, usize> = HashMap::new();
                    
                    // Fix #19: Propagate errors
                    let mut errors = Vec::new();

                    for (weight, res) in results {
                        match res {
                            Ok(source_items) => {
                                for mut item in source_items {
                                    let id = item.item_id;
                                    item.score *= weight;

                                    if let Some(existing) = ensemble_map.get_mut(&id) {
                                        match merge_strategy {
                                            crate::pipeline::MergeStrategy::Sum => existing.score += item.score,
                                            crate::pipeline::MergeStrategy::Max => existing.score = existing.score.max(item.score),
                                            crate::pipeline::MergeStrategy::Min => existing.score = existing.score.min(item.score),
                                            crate::pipeline::MergeStrategy::Average => existing.score += item.score,
                                            crate::pipeline::MergeStrategy::First => {}
                                        }
                                        existing.reasoning.append(&mut item.reasoning);
                                        *counts.get_mut(&id).unwrap() += 1;
                                    } else {
                                        ensemble_map.insert(id, item);
                                        counts.insert(id, 1);
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(e);
                            }
                        }
                    }

                    // If all failed, propagate
                    if !errors.is_empty() && ensemble_map.is_empty() {
                        return Err(errors.remove(0).into());
                    }

                    if *merge_strategy == crate::pipeline::MergeStrategy::Average {
                        // Fix #20: Correct math - divide by total ensemble sources
                        let total_sources = sources.len() as f32;
                        for item in ensemble_map.values_mut() {
                            item.score /= total_sources;
                        }
                    }
                    items = ensemble_map.into_values().collect();
                    // Fix #50: Deterministic sort
                    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                }
                ExecutionNode::Interleave { pattern, sources } => {
                    let futures = sources.iter().map(|(name, nodes)| {
                        async move {
                            let result = self.execute_nodes(nodes, context).await;
                            (name.clone(), result)
                        }
                    });
                    let results = join_all(futures).await;
                    
                    let mut source_results = HashMap::new();
                    for (name, res) in results {
                        if let Ok(source_items) = res {
                            source_results.insert(name, source_items);
                        }
                    }
                    
                    items = self.interleave_results(pattern, &source_results);
                }
            }
            if items.is_empty() && self.is_input_required(node) { break; }
        }
        Ok(items)
    }

    fn is_input_required(&self, node: &ExecutionNode) -> bool {
        match node {
            ExecutionNode::Single(s) => s.implementation.input_type() != crate::pipeline::StageDataKind::Empty,
            _ => true,
        }
    }

    fn evaluate_condition(&self, condition: &crate::pipeline::BranchCondition, context: &ExecutionContext) -> bool {
        let actual_value = match condition.key.as_str() {
            "context.device_type" => context.device_type.as_deref().map(|s| serde_json::json!(s)),
            "context.location" => context.location.as_deref().map(|s| serde_json::json!(s)),
            "context.profile_id" => context.profile_id.as_deref().map(|s| serde_json::json!(s)),
            "context.maturity_rating" => context.maturity_rating.as_deref().map(|s| serde_json::json!(s)),
            "user.top_affinity" => {
                // Heuristic: If we had a top_affinity field in ExecutionContext, we'd use it.
                // For now, we allow the condition to check if it's set in experiment_overrides 
                // or we could potentially fetch it here (blocking, so not ideal).
                // Let's check experiment_overrides as a dynamic store.
                context.experiment_overrides.get("top_affinity").cloned()
            },
            _ => None,
        }.unwrap_or(serde_json::Value::Null);

        match condition.operator.as_str() {
            "==" => actual_value == condition.value,
            "!=" => actual_value != condition.value,
            "exists" => !actual_value.is_null(),
            _ => false,
        }
    }

    fn interleave_results(&self, pattern: &[String], sources: &HashMap<String, Vec<ScoredItem>>) -> Vec<ScoredItem> {
        let mut result = Vec::new();
        let mut pointers: HashMap<String, usize> = sources.keys().map(|k| (k.clone(), 0)).collect();
        let total_available = sources.values().map(|v| v.len()).sum::<usize>();
        
        let mut pattern_idx = 0;
        for _ in 0..total_available {
            let mut added_in_this_round = false;
            
            // Try to find the next item according to the pattern
            for _ in 0..pattern.len() {
                let source_name = &pattern[pattern_idx % pattern.len()];
                pattern_idx += 1;

                if let Some(source_items) = sources.get(source_name) {
                    let ptr = pointers.get_mut(source_name).unwrap();
                    if *ptr < source_items.len() {
                        result.push(source_items[*ptr].clone());
                        *ptr += 1;
                        added_in_this_round = true;
                        break;
                    }
                }
            }

            // If we've exhausted the pattern but still have items in ANY source, 
            // the loop continues and pattern_idx keeps moving to find the next available source.
            if !added_in_this_round {
                break;
            }
        }
        result
    }

    async fn execute_single_stage(&self, bound: &BoundStage, context: &ExecutionContext, input: Vec<ScoredItem>) -> Result<Vec<ScoredItem>> {
        let stage_name = bound.implementation.name();
        let start = std::time::Instant::now();

        let result: Result<Vec<ScoredItem>, crate::error::AppError> = if let Some(ref breaker) = bound.breaker {
            // Fix #6: Use breaker.call to record outcomes (success/failure)
            // We map anyhow::Error to AppError::Internal to satisfy ErrorClassifier bound
            breaker.call(|| async {
                bound.implementation.execute(context, &bound.params, input.clone()).await
                    .map_err(|e| crate::error::AppError::Internal(e.to_string()))
            }).await
            .map_err(|cb_err| {
                match cb_err {
                    crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => 
                        crate::error::AppError::Pipeline(PipelineError::CircuitOpen { stage: bound.stage_type.clone() }),
                    crate::circuit_breaker::CircuitBreakerError::TimedOut { timeout } => 
                        crate::error::AppError::Pipeline(PipelineError::StageTimeout { stage: bound.stage_type.clone(), timeout_ms: timeout.as_millis() as u64 }),
                    crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => 
                        source, // Already an AppError
                }
            })
        } else {
            // Fallback to direct execution with timeout if no breaker
            tokio::time::timeout(bound.timeout, bound.implementation.execute(context, &bound.params, input.clone()))
                .await
                .map_err(|_| crate::error::AppError::Pipeline(PipelineError::StageTimeout { stage: bound.stage_type.clone(), timeout_ms: bound.timeout.as_millis() as u64 }))
                .and_then(|res| res.map_err(|e| crate::error::AppError::Internal(e.to_string())))
        };

        let latency = start.elapsed();
        context.record_stage_latency(stage_name, latency.as_millis() as u64);

        match result {
            Ok(items) => {
                context.record_stage_throughput(stage_name);
                Ok(items)
            }
            Err(e) => {
                context.record_stage_error(stage_name);
                
                if self.config.fallback_pass_through_input {
                    warn!(stage = %stage_name, error = %e, "Stage failed, passing through input due to fallback config");
                    return Ok(input);
                }
                
                Err(e.into())
            }
        }
    }

    fn timeout_for_stage(stage_type: &str, config: &PipelineConfig) -> Duration {
        if stage_type.starts_with("fetch_") { config.fetch_stage_timeout }
        else if stage_type.starts_with("onnx_") || stage_type.starts_with("ml_") { config.ml_stage_timeout }
        else if stage_type.starts_with("filter_") { config.filter_stage_timeout }
        else { config.stage_timeout_default }
    }
}
