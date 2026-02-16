//! Netflix-grade pipeline executor with per-stage resilience.
//!
//! Every stage execution is:
//! 1. Guarded by a per-stage circuit breaker (trips on failure/slow calls)
//! 2. Bounded by a configurable timeout (per stage category)
//! 3. Recorded in analytics (latency, throughput, errors per stage)
//! 4. Wrapped with fallback (pass-through input on failure if configured)
//! 5. Classified via `PipelineError` → `ErrorClassifier` for resilience routing

use anyhow::{Result, Context};

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, error, debug};
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
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::registry::build_stage_registry;
use crate::db::models::PipelineDefinition;

/// Netflix-grade pipeline executor with per-stage resilience.
pub struct PipelineExecutor {
    /// Registry of all available stages.
    stage_registry: HashMap<String, Arc<dyn PipelineStage>>,

    /// Per-stage circuit breakers keyed by stage type name.
    stage_breakers: HashMap<String, Arc<CircuitBreaker>>,

    /// Structural validator for pipelines.
    validator: PipelineValidator,

    /// Pipeline resilience configuration.
    config: PipelineConfig,

    /// Analytics for pipeline-level metrics.
    analytics: Option<Arc<PerformanceStats>>,
}

impl PipelineExecutor {
    /// Create new pipeline executor with full resilience wiring.
    pub fn new(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
    ) -> Self {
        Self::with_registry(config, breaker_registry, observer, analytics, build_stage_registry())
    }

    /// Create new pipeline executor with a custom registry (useful for tests/benches).
    pub fn with_registry(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
        registry: HashMap<String, Arc<dyn PipelineStage>>,
    ) -> Self {
        let validator = PipelineValidator::new(registry.clone());
        // Build per-stage circuit breakers
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

                match breaker_config {
                    Ok(bc) => {
                        // Leak the breaker ID — stage names are long-lived singletons
                        let breaker_id: &'static str = Box::leak(
                            format!("pipeline.stage.{}", stage_name).into_boxed_str()
                        );
                        let breaker = Arc::new(CircuitBreaker::new(
                            CircuitBreakerId::new(breaker_id),
                            bc,
                            observer.clone(),
                        ));
                        breaker_registry.register(breaker.clone());
                        stage_breakers.insert(stage_name.clone(), breaker);
                    }
                    Err(e) => {
                        warn!(
                            stage = %stage_name,
                            error = %e,
                            "Failed to build circuit breaker for stage, running without breaker"
                        );
                    }
                }
            }
        }

        info!(
            stage_count = registry.len(),
            breaker_count = stage_breakers.len(),
            "Pipeline executor initialized with resilience"
        );

        Self {
            stage_registry: registry,
            stage_breakers,
            validator,
            config,
            analytics,
        }
    }

    /// Link a pipeline definition into an executable version.
    /// This performs structural validation, registry lookups, and pre-calculates circuit breakers and timeouts.
    /// It also automatically identifies independent stages and groups them into parallel execution nodes.
    pub fn link(&self, definition: &PipelineDefinition) -> Result<ExecutablePipeline> {
        // Step 1: Structural Validation (Safety Gate)
        self.validator.validate_definition(definition)?;

        // Step 2: Linking & Grouping into Execution Nodes
        let nodes = self.link_to_nodes(&definition.stages)?;
        let fallback_nodes = if let Some(ref fallback) = definition.fallback_stages {
            Some(self.link_to_nodes(fallback)?)
        } else {
            None
        };

        Ok(ExecutablePipeline {
            nodes,
            fallback_nodes,
        })
    }

    fn link_to_nodes(&self, stages: &[crate::db::models::PipelineStageConfig]) -> Result<Vec<ExecutionNode>> {
        let mut nodes = Vec::new();
        let mut current_parallel_group = Vec::new();

        for stage_config in stages {
            let implementation = self.stage_registry
                .get(&stage_config.r#type)
                .cloned()
                .with_context(|| format!("Stage '{}' not found in registry", stage_config.r#type))?;

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
                current_parallel_group.push(bound);
            } else {
                // If we have a parallel group accumulated, push it first
                if !current_parallel_group.is_empty() {
                    if current_parallel_group.len() == 1 {
                        nodes.push(ExecutionNode::Single(current_parallel_group.pop().unwrap()));
                    } else {
                        nodes.push(ExecutionNode::Parallel(current_parallel_group));
                        current_parallel_group = Vec::new();
                    }
                }
                // Push the sync barrier stage
                nodes.push(ExecutionNode::Single(bound));
            }
        }

        // Push any remaining parallel group
        if !current_parallel_group.is_empty() {
            if current_parallel_group.len() == 1 {
                nodes.push(ExecutionNode::Single(current_parallel_group.pop().unwrap()));
            } else {
                nodes.push(ExecutionNode::Parallel(current_parallel_group));
            }
        }

        Ok(nodes)
    }

    /// Get the number of registered stages.
    pub fn stage_count(&self) -> usize {
        self.stage_registry.len()
    }

    /// Pipeline config accessor.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Execute a pipeline definition with full resilience.
    pub async fn execute(
        &self,
        pipeline: &PipelineDefinition,
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        let linked = self.link(pipeline)?;
        self.execute_linked(&linked, context).await
    }

    /// Execute a pre-linked pipeline with full resilience.
    pub async fn execute_linked(
        &self,
        pipeline: &ExecutablePipeline,
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        let pipeline_start = std::time::Instant::now();
        let metric_key = "pipeline.execute";

        info!(
            request_id = %context.request_id,
            node_count = pipeline.nodes.len(),
            "Executing linked pipeline"
        );

        // Execute main pipeline with pipeline-level timeout
        let pipeline_timeout = self.config.pipeline_timeout;
        let result = tokio::time::timeout(
            pipeline_timeout,
            self.execute_nodes(&pipeline.nodes, context),
        ).await;

        let pipeline_latency = pipeline_start.elapsed();

        match result {
            Ok(Ok(items)) => {
                // Record success analytics
                if let Some(ref a) = self.analytics {
                    a.record_response_time(metric_key, pipeline_latency.as_millis() as u64);
                    a.increment_throughput(metric_key);
                }

                info!(
                    request_id = %context.request_id,
                    result_count = items.len(),
                    pipeline_ms = pipeline_latency.as_millis() as u64,
                    "Pipeline executed successfully"
                );

                Ok(items)
            }
            Ok(Err(e)) => {
                error!(
                    request_id = %context.request_id,
                    error = %e,
                    pipeline_ms = pipeline_latency.as_millis() as u64,
                    "Main pipeline failed"
                );

                if let Some(ref a) = self.analytics {
                    a.record_response_time(metric_key, pipeline_latency.as_millis() as u64);
                    a.increment_error(metric_key);
                }

                // Execute fallback pipeline if available
                if let Some(ref fallback_nodes) = pipeline.fallback_nodes {
                    if self.config.fallback_enabled {
                        warn!(
                            request_id = %context.request_id,
                            fallback_node_count = fallback_nodes.len(),
                            "Executing fallback pipeline"
                        );
                        if let Some(ref a) = self.analytics {
                            a.increment_throughput("pipeline.fallback");
                        }
                        self.execute_nodes(fallback_nodes, context).await
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
            Err(_elapsed) => {
                error!(
                    request_id = %context.request_id,
                    timeout_ms = pipeline_timeout.as_millis() as u64,
                    "Pipeline execution timed out"
                );

                if let Some(ref a) = self.analytics {
                    a.record_response_time(metric_key, pipeline_timeout.as_millis() as u64);
                    a.increment_error(&format!("{}.timeout", metric_key));
                }

                // Fallback on pipeline timeout
                if let Some(ref fallback_nodes) = pipeline.fallback_nodes {
                    if self.config.fallback_enabled {
                        warn!(
                            request_id = %context.request_id,
                            "Pipeline timed out, executing fallback"
                        );
                        self.execute_nodes(fallback_nodes, context).await
                    } else {
                        Err(PipelineError::PipelineTimeout.into())
                    }
                } else {
                    Err(PipelineError::PipelineTimeout.into())
                }
            }
        }
    }

    /// Execute a sequence of execution nodes with per-node resilience.
    async fn execute_nodes(
        &self,
        nodes: &[ExecutionNode],
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        let mut items: Vec<ScoredItem> = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            match node {
                ExecutionNode::Single(bound_stage) => {
                    items = self.execute_single_stage(bound_stage, context, items).await?;
                }
                ExecutionNode::Parallel(stages) => {
                    debug!(
                        request_id = %context.request_id,
                        node_idx = idx,
                        parallel_count = stages.len(),
                        "Executing parallel node"
                    );

                    let mut futures = Vec::with_capacity(stages.len());
                    for stage in stages {
                        futures.push(self.execute_single_stage(stage, context, items.clone()));
                    }

                    let results = join_all(futures).await;
                    
                    let mut merged_map: std::collections::HashMap<i32, ScoredItem> = std::collections::HashMap::new();

                    for res in results {
                        match res {
                            Ok(stage_items) => {
                                for item in stage_items {
                                    match merged_map.entry(item.item_id) {
                                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                                            // Merge metadata if it's an object
                                            if let (Some(dest), Some(src)) = (entry.get_mut().metadata.as_object_mut(), item.metadata.as_object()) {
                                                for (k, v) in src {
                                                    dest.insert(k.clone(), v.clone());
                                                }
                                            }
                                            // Optional: update score if needed (taking max or avg), 
                                            // but for enrichment, scores are usually stable.
                                        }
                                        std::collections::hash_map::Entry::Vacant(entry) => {
                                            entry.insert(item);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    request_id = %context.request_id,
                                    error = %e,
                                    "Parallel stage failed, continuing with other results"
                                );
                            }
                        }
                    }
                    items = merged_map.into_values().collect();
                }
            }

            // Early termination if no items remain
            if items.is_empty() {
                warn!(
                    request_id = %context.request_id,
                    node_idx = idx,
                    "Pipeline terminated early: no items remaining"
                );
                break;
            }
        }

        Ok(items)
    }

    /// Internal helper to execute a single bound stage with full resilience.
    async fn execute_single_stage(
        &self,
        bound_stage: &BoundStage,
        context: &ExecutionContext,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let stage_impl = &bound_stage.implementation;
        let stage_name = stage_impl.name();
        let stage_start = std::time::Instant::now();
        let stage_timeout = bound_stage.timeout;

        debug!(
            request_id = %context.request_id,
            stage_type = %bound_stage.stage_type,
            input_count = input.len(),
            "Executing bound stage"
        );

        // Check circuit breaker state before executing
        if let Some(ref breaker) = bound_stage.breaker {
            if breaker.current_state() == CircuitState::Open {
                warn!(
                    request_id = %context.request_id,
                    stage = %bound_stage.stage_type,
                    "Circuit breaker open, skipping stage"
                );
                // Fallback: pass-through input on circuit open
                if self.config.fallback_pass_through_input {
                    return Ok(input);
                }
                return Err(PipelineError::CircuitOpen { stage_type: bound_stage.stage_type.clone() }.into());
            }
        }

        // Execute stage with timeout
        let stage_result = tokio::time::timeout(
            stage_timeout,
            stage_impl.execute(context, &bound_stage.params, input.clone()),
        ).await;

        let stage_latency = stage_start.elapsed();

        match stage_result {
            Ok(Ok(result_items)) => {
                // Record analytics
                context.record_stage_latency(stage_name, stage_latency.as_millis() as u64);
                context.record_stage_throughput(stage_name);
                Ok(result_items)
            }
            Ok(Err(e)) => {
                // Record error analytics
                context.record_stage_error(stage_name);
                
                warn!(
                    request_id = %context.request_id,
                    stage_type = %bound_stage.stage_type,
                    error = %e,
                    "Stage failed"
                );

                // Fallback: pass-through input on stage error
                if self.config.fallback_on_stage_error && self.config.fallback_pass_through_input {
                    return Ok(input);
                }

                Err(PipelineError::StageFailure { 
                    stage_type: bound_stage.stage_type.clone(), 
                    source: e 
                }.into())
            }
            Err(_elapsed) => {
                // Record timeout analytics
                context.record_stage_error(stage_name);

                warn!(
                    request_id = %context.request_id,
                    stage_type = %bound_stage.stage_type,
                    timeout_ms = stage_timeout.as_millis() as u64,
                    "Stage timed out"
                );

                // Fallback: pass-through input on stage timeout
                if self.config.fallback_on_stage_timeout && self.config.fallback_pass_through_input {
                    return Ok(input);
                }

                Err(PipelineError::StageTimeout { 
                    stage_type: bound_stage.stage_type.clone(), 
                    timeout_ms: stage_timeout.as_millis() as u64 
                }.into())
            }
        }
    }

    /// Determine the timeout for a given stage based on its category.
    fn timeout_for_stage(stage_type: &str, config: &PipelineConfig) -> Duration {
        if stage_type.starts_with("fetch_") {
            config.fetch_stage_timeout
        } else if stage_type.starts_with("onnx_") || stage_type.starts_with("ml_") {
            config.ml_stage_timeout
        } else if stage_type.starts_with("filter_") {
            config.filter_stage_timeout
        } else {
            config.stage_timeout_default
        }
    }
}
