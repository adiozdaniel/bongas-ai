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

use crate::analytics::types::PerformanceStats;
use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig as BreakerConfig,
    CircuitBreakerRegistry, CircuitBreakerId,
};
use crate::circuit_breaker::observer::{CircuitState, ResilienceObserver};
use crate::config::PipelineConfig;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::registry::build_stage_registry;
use crate::db::models::PipelineDefinition;

/// Netflix-grade pipeline executor with per-stage resilience.
pub struct PipelineExecutor {
    /// Registry of all available stages.
    stage_registry: HashMap<String, Arc<dyn PipelineStage>>,

    /// Per-stage circuit breakers keyed by stage type name.
    stage_breakers: HashMap<String, Arc<CircuitBreaker>>,

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
            config,
            analytics,
        }
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
        let pipeline_start = std::time::Instant::now();
        let metric_key = "pipeline.execute";

        info!(
            request_id = %context.request_id,
            stage_count = pipeline.stages.len(),
            "Executing pipeline"
        );

        // Log ONNX inference stages
        let onnx_stages = pipeline.stages.iter()
            .filter(|stage| stage.r#type.starts_with("onnx_"))
            .count();

        if onnx_stages > 0 {
            info!(
                request_id = %context.request_id,
                onnx_stage_count = onnx_stages,
                "Pipeline contains ONNX inference stages"
            );
        }

        // Execute main pipeline with pipeline-level timeout
        let pipeline_timeout = self.config.pipeline_timeout;
        let result = tokio::time::timeout(
            pipeline_timeout,
            self.execute_stages(&pipeline.stages, context),
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
                if let Some(ref fallback_stages) = pipeline.fallback_stages {
                    if self.config.fallback_enabled {
                        warn!(
                            request_id = %context.request_id,
                            fallback_stage_count = fallback_stages.len(),
                            "Executing fallback pipeline"
                        );
                        if let Some(ref a) = self.analytics {
                            a.increment_throughput("pipeline.fallback");
                        }
                        self.execute_stages(fallback_stages, context).await
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
                if let Some(ref fallback_stages) = pipeline.fallback_stages {
                    if self.config.fallback_enabled {
                        warn!(
                            request_id = %context.request_id,
                            "Pipeline timed out, executing fallback"
                        );
                        self.execute_stages(fallback_stages, context).await
                    } else {
                        Err(anyhow::anyhow!("Pipeline execution timed out after {}ms", pipeline_timeout.as_millis()))
                    }
                } else {
                    Err(anyhow::anyhow!("Pipeline execution timed out after {}ms", pipeline_timeout.as_millis()))
                }
            }
        }
    }

    /// Execute a sequence of stages with per-stage resilience.
    async fn execute_stages(
        &self,
        stages: &[crate::db::models::PipelineStageConfig],
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        let mut items: Vec<ScoredItem> = Vec::new();

        for (idx, stage_config) in stages.iter().enumerate() {
            let stage_impl = self.stage_registry
                .get(&stage_config.r#type)
                .with_context(|| format!("Stage '{}' not found in registry", stage_config.r#type))?;

            let stage_name = stage_impl.name();
            let stage_start = std::time::Instant::now();
            let stage_timeout = Self::timeout_for_stage(&stage_config.r#type, &self.config);

            debug!(
                request_id = %context.request_id,
                stage_idx = idx,
                stage_type = %stage_config.r#type,
                stage_name = stage_name,
                input_count = items.len(),
                timeout_ms = stage_timeout.as_millis() as u64,
                "Executing stage"
            );

            // Check circuit breaker state before executing
            if let Some(breaker) = self.stage_breakers.get(&stage_config.r#type) {
                if breaker.current_state() == CircuitState::Open {
                    warn!(
                        request_id = %context.request_id,
                        stage = %stage_config.r#type,
                        "Circuit breaker open, skipping stage"
                    );
                    if let Some(ref a) = self.analytics {
                        a.increment_error(&format!("pipeline.stage.{}.circuit_open", stage_config.r#type));
                    }
                    // Fallback: pass-through input on circuit open
                    if self.config.fallback_pass_through_input {
                        continue;
                    }
                    return Err(anyhow::anyhow!(
                        "Circuit breaker open for stage '{}'", stage_config.r#type
                    ));
                }
            }

            // Execute stage with timeout
            let stage_result = tokio::time::timeout(
                stage_timeout,
                stage_impl.execute(context, &stage_config.params, items.clone()),
            ).await;

            let stage_latency = stage_start.elapsed();

            match stage_result {
                Ok(Ok(result_items)) => {
                    // Record analytics
                    context.record_stage_latency(stage_name, stage_latency.as_millis() as u64);
                    context.record_stage_throughput(stage_name);

                    info!(
                        request_id = %context.request_id,
                        stage_idx = idx,
                        stage_type = %stage_config.r#type,
                        output_count = result_items.len(),
                        duration_ms = stage_latency.as_millis() as u64,
                        "Stage completed"
                    );

                    items = result_items;
                }
                Ok(Err(e)) => {
                    // Record error analytics
                    context.record_stage_error(stage_name);
                    context.record_stage_latency(stage_name, stage_latency.as_millis() as u64);

                    warn!(
                        request_id = %context.request_id,
                        stage_idx = idx,
                        stage_type = %stage_config.r#type,
                        error = %e,
                        duration_ms = stage_latency.as_millis() as u64,
                        "Stage failed"
                    );

                    // Fallback: pass-through input on stage error
                    if self.config.fallback_on_stage_error && self.config.fallback_pass_through_input {
                        warn!(
                            request_id = %context.request_id,
                            stage = %stage_config.r#type,
                            "Using pass-through fallback for failed stage"
                        );
                        if let Some(ref a) = self.analytics {
                            a.increment_throughput(&format!("pipeline.stage.{}.fallback", stage_config.r#type));
                        }
                        continue; // items unchanged, skip failed stage
                    }

                    return Err(e).with_context(|| format!(
                        "Stage '{}' (index {}) failed", stage_config.r#type, idx
                    ));
                }
                Err(_elapsed) => {
                    // Record timeout analytics
                    context.record_stage_error(stage_name);
                    if let Some(ref a) = self.analytics {
                        a.increment_error(&format!("pipeline.stage.{}.timeout", stage_config.r#type));
                    }

                    warn!(
                        request_id = %context.request_id,
                        stage_idx = idx,
                        stage_type = %stage_config.r#type,
                        timeout_ms = stage_timeout.as_millis() as u64,
                        "Stage timed out"
                    );

                    // Fallback: pass-through input on stage timeout
                    if self.config.fallback_on_stage_timeout && self.config.fallback_pass_through_input {
                        warn!(
                            request_id = %context.request_id,
                            stage = %stage_config.r#type,
                            "Using pass-through fallback for timed-out stage"
                        );
                        if let Some(ref a) = self.analytics {
                            a.increment_throughput(&format!("pipeline.stage.{}.timeout_fallback", stage_config.r#type));
                        }
                        continue;
                    }

                    return Err(anyhow::anyhow!(
                        "Stage '{}' (index {}) timed out after {}ms",
                        stage_config.r#type, idx, stage_timeout.as_millis()
                    ));
                }
            }

            // Early termination if no items remain
            if items.is_empty() {
                warn!(
                    request_id = %context.request_id,
                    stage_idx = idx,
                    stage_type = %stage_config.r#type,
                    "Pipeline terminated early: no items remaining"
                );
                break;
            }
        }

        Ok(items)
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
