//! Netflix-grade pipeline executor with per-stage resilience and automatic parallelization.

use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tracing::warn;
use futures::future::join_all;

use crate::analytics::types::PerformanceStats;
use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig as BreakerConfig,
    CircuitBreakerRegistry, CircuitBreakerId,
};
use crate::circuit_breaker::observer::{CircuitState, ResilienceObserver};
use crate::config::PipelineConfig;
use crate::pipeline::{PipelineStage, ScoredItem, BoundStage, ExecutionNode, ExecutablePipeline, PipelineError};
use crate::pipeline::validator::PipelineValidator;
use crate::pipeline::optimizer::PipelineOptimizer;
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::registry::build_stage_registry;
use crate::db::models::PipelineDefinition;

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
                    let breaker_id: &'static str = Box::leak(format!("pipeline.stage.{}", stage_name).into_boxed_str());
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
                    nodes.push(ExecutionNode::Ensemble { sources });
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
            ExecutionNode::Parallel(std::mem::take(current_parallel))
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
                    Err(PipelineError::PipelineTimeout.into())
                }
            }
        }
    }

    async fn execute_nodes(&self, nodes: &[ExecutionNode], context: &ExecutionContext) -> Result<Vec<ScoredItem>> {
        let mut items = Vec::new();
        for node in nodes {
            match node {
                ExecutionNode::Single(stage) => {
                    items = self.execute_single_stage(stage, context, items).await?;
                }
                ExecutionNode::Fused(stages) => {
                    // JIT-lite: Fused Scoring Pass
                    if !items.is_empty() {
                        for stage in stages {
                            items = self.execute_single_stage(stage, context, items).await?;
                        }
                    }
                }
                ExecutionNode::Parallel(stages) => {
                    let futures = stages.iter().map(|s| self.execute_single_stage(s, context, items.clone()));
                    let results = join_all(futures).await;
                    let mut merged_map = HashMap::new();
                    for res in results {
                        if let Ok(stage_items) = res {
                            for item in stage_items {
                                merged_map.entry(item.item_id).and_modify(|existing: &mut ScoredItem| {
                                    if let (Some(dest), Some(src)) = (existing.metadata.as_object_mut(), item.metadata.as_object()) {
                                        for (k, v) in src { dest.insert(k.clone(), v.clone()); }
                                    }
                                }).or_insert(item);
                            }
                        }
                    }
                    items = merged_map.into_values().collect();
                }
                ExecutionNode::Branch { condition, if_true, if_false } => {
                    let branch = if self.evaluate_condition(condition, context) {
                        if_true
                    } else {
                        if_false
                    };
                    // Use Box::pin to handle recursion in async function
                    let fut = self.execute_nodes(branch, context);
                    items = Box::pin(fut).await?;
                }
                ExecutionNode::Ensemble { sources } => {
                    let futures = sources.iter().map(|source| {
                        async move {
                            let result = Box::pin(self.execute_nodes(&source.nodes, context)).await;
                            (source.weight, result)
                        }
                    });
                    let results = join_all(futures).await;
                    
                    let mut ensemble_map: HashMap<i32, ScoredItem> = HashMap::new();
                    for (weight, res) in results {
                        if let Ok(source_items) = res {
                            for mut item in source_items {
                                item.score *= weight;
                                ensemble_map.entry(item.item_id)
                                    .and_modify(|e| e.score += item.score)
                                    .or_insert(item);
                            }
                        }
                    }
                    items = ensemble_map.into_values().collect();
                }
                ExecutionNode::Interleave { pattern, sources } => {
                    let futures = sources.iter().map(|(name, nodes)| {
                        async move {
                            let result = Box::pin(self.execute_nodes(nodes, context)).await;
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
        let max_items = sources.values().map(|v| v.len()).sum::<usize>();

        for _ in 0..max_items {
            let mut added = false;
            for source_name in pattern {
                if let Some(source_items) = sources.get(source_name) {
                    let ptr = pointers.get_mut(source_name).unwrap();
                    if *ptr < source_items.len() {
                        result.push(source_items[*ptr].clone());
                        *ptr += 1;
                        added = true;
                        break;
                    }
                }
            }
            if !added { break; }
        }
        result
    }

    async fn execute_single_stage(&self, bound: &BoundStage, context: &ExecutionContext, input: Vec<ScoredItem>) -> Result<Vec<ScoredItem>> {
        let stage_name = bound.implementation.name();
        let start = std::time::Instant::now();

        if let Some(ref breaker) = bound.breaker {
            if breaker.current_state() == CircuitState::Open {
                if self.config.fallback_pass_through_input { return Ok(input); }
                return Err(PipelineError::CircuitOpen { stage_type: bound.stage_type.clone() }.into());
            }
        }

        let result = tokio::time::timeout(bound.timeout, bound.implementation.execute(context, &bound.params, input.clone())).await;
        let latency = start.elapsed();
        context.record_stage_latency(stage_name, latency.as_millis() as u64);

        match result {
            Ok(Ok(items)) => {
                context.record_stage_throughput(stage_name);
                Ok(items)
            }
            Ok(Err(e)) => {
                context.record_stage_error(stage_name);
                if self.config.fallback_on_stage_error && self.config.fallback_pass_through_input { return Ok(input); }
                Err(PipelineError::StageFailure { stage_type: bound.stage_type.clone(), source: e }.into())
            }
            Err(_) => {
                context.record_stage_error(stage_name);
                if self.config.fallback_on_stage_timeout && self.config.fallback_pass_through_input { return Ok(input); }
                Err(PipelineError::StageTimeout { stage_type: bound.stage_type.clone(), timeout_ms: bound.timeout.as_millis() as u64 }.into())
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
