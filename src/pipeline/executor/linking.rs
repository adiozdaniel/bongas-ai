use anyhow::{Result, Context};
use std::time::Duration;
use crate::config::PipelineConfig;
use crate::pipeline::{BoundStage, ExecutionNode, ExecutablePipeline};
use crate::pipeline::optimizer::service::PipelineOptimizer;
use crate::db::{PipelineDefinition, PipelineStageConfig};
use crate::pipeline::executor::service::PipelineExecutor;

impl PipelineExecutor {
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

    pub(super) fn link_to_nodes(&self, configs: &[PipelineStageConfig]) -> Result<Vec<ExecutionNode>> {
        let mut nodes = Vec::new();
        for config in configs {
            let stage_impl = self.registry.get(&config.r#type)
                .context(format!("Stage type '{}' not found in registry", config.r#type))?;
            
            let stage_type = config.r#type.clone();
            let timeout = Self::timeout_for_stage(&stage_type, &self.config);

            nodes.push(ExecutionNode::Single(BoundStage {
                implementation: stage_impl,
                params: config.params.clone(),
                breaker: self.stage_breakers.get(&config.r#type).cloned(),
                stage_type,
                timeout,
            }));
        }
        Ok(nodes)
    }

    pub(super) fn timeout_for_stage(name: &str, config: &PipelineConfig) -> Duration {
        match name {
            n if n.starts_with("fetch_") => config.stage_timeout_default, // Correct field name
            n if n.starts_with("ml_") || n.contains("candle") => config.ml_stage_timeout,
            _ => config.stage_timeout_default,
        }
    }
}
