use anyhow::Result;
use std::time::Instant;
use tracing::warn;
use async_recursion::async_recursion;

use crate::pipeline::{ScoredItem, ExecutionNode, ExecutablePipeline, BoundStage};
use crate::pipeline::context::ExecutionContext;
use crate::db::models::PipelineDefinition;
use crate::error::{PipelineError, AppError};
use crate::pipeline::executor::service::PipelineExecutor;

impl PipelineExecutor {
    pub async fn execute(
        &self,
        definition: &PipelineDefinition,
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        let pipeline = self.link(definition)?;
        self.execute_linked(&pipeline, context).await
    }

    pub async fn execute_linked(
        &self,
        pipeline: &ExecutablePipeline,
        context: &ExecutionContext,
    ) -> Result<Vec<ScoredItem>> {
        match self.execute_nodes(&pipeline.nodes, context, Vec::new()).await {
            Ok(results) => Ok(results),
            Err(e) if pipeline.fallback_nodes.is_some() => {
                warn!("Pipeline main path failed, executing fallback: {}", e);
                self.execute_nodes(pipeline.fallback_nodes.as_ref().unwrap(), context, Vec::new()).await
            }
            Err(e) => Err(e),
        }
    }

    #[async_recursion]
    pub(super) async fn execute_nodes(
        &self,
        nodes: &[ExecutionNode],
        context: &ExecutionContext,
        initial_data: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let mut current_data = initial_data;

        for node in nodes {
            match node {
                ExecutionNode::Single(stage) => {
                    current_data = self.execute_bound_stage(stage, context, current_data).await?;
                }
                // Handle other variants as needed
                _ => {
                    warn!("ExecutionNode variant not yet implemented in simplified executor");
                }
            }
        }

        Ok(current_data)
    }

    async fn execute_bound_stage(
        &self,
        node: &BoundStage,
        context: &ExecutionContext,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let start = Instant::now();
        let stage_name = node.stage_type.clone();

        // Execute with circuit breaker if present
        let result = if let Some(ref breaker) = node.breaker {
            let stage = node.implementation.clone();
            let params = node.params.clone();
            let data = input.clone();
            
            // Map anyhow::Error to AppError so the circuit breaker can classify it
            breaker.call(move || {
                let stage = stage.clone();
                let params = params.clone();
                let data = data.clone();
                async move {
                    stage.execute(context, &params, data).await
                        .map_err(AppError::from)
                }
            }).await
              .map_err(|_| PipelineError::CircuitOpen { stage: stage_name.clone() }.into())
        } else {
            node.implementation.execute(context, &node.params, input).await
        };

        let duration = start.elapsed();
        
        // Record analytics
        if let Some(ref stats) = self.analytics {
            let metric_key = format!("pipeline.stage.{}", stage_name);
            stats.record_response_time(&metric_key, duration.as_millis() as u64);
            if result.is_ok() {
                stats.increment_throughput(&metric_key);
            } else {
                stats.increment_error(&metric_key);
            }
        }

        // Return Result<Vec<ScoredItem>, anyhow::Error>
        match result {
            Ok(data) => Ok(data),
            Err(e) => Err(e.into()),
        }
    }
}
