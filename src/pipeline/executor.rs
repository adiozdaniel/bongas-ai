use anyhow::{Result, Context};

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::registry::build_stage_registry;
use crate::db::models::PipelineDefinition;

pub struct PipelineExecutor {
    /// Registry of all available stages
    stage_registry: HashMap<String, Arc<dyn PipelineStage>>,
}

impl PipelineExecutor {
    pub fn new() -> Self {
        let registry = build_stage_registry();

        info!(
            stage_count = registry.len(),
            "Pipeline executor initialized with registered stages"
        );

        Self {
            stage_registry: registry,
        }
    }

    /// Get the number of registered stages
    pub fn stage_count(&self) -> usize {
        self.stage_registry.len()
    }

    /// Execute a pipeline definition
    pub async fn execute(
        &self,
        pipeline: &PipelineDefinition,
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
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

        // Execute main pipeline
        match self.execute_stages(&pipeline.stages, context).await {
            Ok(results) => Ok(results),
            Err(e) => {
                error!(
                    request_id = %context.request_id,
                    error = %e,
                    "Main pipeline failed"
                );

                // Execute fallback pipeline if available
                if let Some(ref fallback_stages) = pipeline.fallback_stages {
                    warn!(
                        request_id = %context.request_id,
                        fallback_stage_count = fallback_stages.len(),
                        "Executing fallback pipeline"
                    );
                    self.execute_stages(fallback_stages, context).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Execute a sequence of stages
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

            info!(
                request_id = %context.request_id,
                stage_idx = idx,
                stage_type = %stage_config.r#type,
                stage_name = stage_impl.name(),
                input_count = items.len(),
                "Executing stage"
            );

            let start = std::time::Instant::now();

            items = stage_impl.execute(context, &stage_config.params, items).await
                .with_context(|| format!("Stage '{}' (index {}) failed", stage_config.r#type, idx))?;

            let duration = start.elapsed();

            info!(
                request_id = %context.request_id,
                stage_idx = idx,
                stage_type = %stage_config.r#type,
                output_count = items.len(),
                duration_ms = duration.as_millis() as u64,
                "Stage completed"
            );

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
}
