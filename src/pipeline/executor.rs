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
                    nodes.push(if current_parallel.len() == 1 {
                        ExecutionNode::Single(current_parallel.pop().unwrap())
                    } else {
                        ExecutionNode::Parallel(std::mem::take(&mut current_parallel))
                    });
                }
                nodes.push(ExecutionNode::Single(bound));
            }
        }

        if !current_parallel.is_empty() {
            nodes.push(if current_parallel.len() == 1 {
                ExecutionNode::Single(current_parallel.pop().unwrap())
            } else {
                ExecutionNode::Parallel(current_parallel)
            });
        }

        Ok(nodes)
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
            }
            if items.is_empty() { break; }
        }
        Ok(items)
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
