use anyhow::Result;
use std::time::Instant;
use tracing::warn;
use async_recursion::async_recursion;
use std::collections::HashMap;
use futures::future::join_all;

use crate::pipeline::{ScoredItem, ExecutionNode, ExecutablePipeline, BoundStage, MergeStrategy, EnsembleSource};
use crate::pipeline::context::service::ExecutionContext;
use crate::db::PipelineDefinition;
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
                ExecutionNode::Parallel { stages, merge_strategy } => {
                    let mut futures = Vec::new();
                    for stage in stages {
                        futures.push(self.execute_bound_stage(stage, context, current_data.clone()));
                    }
                    let results = join_all(futures).await;
                    
                    let mut successful_results = Vec::new();
                    for res in results {
                        match res {
                            Ok(data) => successful_results.push(data),
                            Err(e) => warn!("Parallel stage failed: {}", e),
                        }
                    }
                    
                    if successful_results.is_empty() {
                        return Err(anyhow::anyhow!("All parallel stages failed"));
                    }
                    
                    current_data = self.merge_results(successful_results, *merge_strategy);
                }
                ExecutionNode::Fused(stages) => {
                    for stage in stages {
                        current_data = self.execute_bound_stage(stage, context, current_data).await?;
                    }
                }
                ExecutionNode::Branch { condition, if_true, if_false } => {
                    let matches = self.evaluate_condition(condition, context);
                    let branch_nodes = if matches { if_true } else { if_false };
                    current_data = self.execute_nodes(branch_nodes, context, current_data).await?;
                }
                ExecutionNode::Ensemble { sources, merge_strategy } => {
                    let mut futures = Vec::new();
                    for source in sources {
                        futures.push(self.execute_ensemble_source(source, context, current_data.clone()));
                    }
                    let results = join_all(futures).await;
                    
                    let mut successful_results = Vec::new();
                    for res in results {
                        match res {
                            Ok(data) => successful_results.push(data),
                            Err(e) => warn!("Ensemble branch failed: {}", e),
                        }
                    }
                    
                    if successful_results.is_empty() {
                        return Err(anyhow::anyhow!("All ensemble branches failed"));
                    }
                    
                    current_data = self.merge_results(successful_results, *merge_strategy);
                }
                ExecutionNode::Interleave { pattern, sources } => {
                    let mut branch_results = HashMap::new();
                    for (name, nodes) in sources {
                        let res = self.execute_nodes(nodes, context, current_data.clone()).await?;
                        branch_results.insert(name.clone(), res);
                    }
                    current_data = self.interleave_results(pattern, branch_results);
                }
            }
        }

        Ok(current_data)
    }

    async fn execute_ensemble_source(
        &self,
        source: &EnsembleSource,
        context: &ExecutionContext,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let mut results = self.execute_nodes(&source.nodes, context, input).await?;
        for item in &mut results {
            item.score *= source.weight;
        }
        Ok(results)
    }

    fn merge_results(&self, mut results: Vec<Vec<ScoredItem>>, strategy: MergeStrategy) -> Vec<ScoredItem> {
        if results.is_empty() { return Vec::new(); }
        if results.len() == 1 { return results.remove(0); }

        let mut item_map: HashMap<i32, ScoredItem> = HashMap::new();
        let mut counts: HashMap<i32, usize> = HashMap::new();

        for batch in results {
            for item in batch {
                let entry = item_map.entry(item.item_id).or_insert_with(|| {
                    ScoredItem::new(item.item_id, 0.0, item.metadata.clone())
                });
                
                *counts.entry(item.item_id).or_insert(0) += 1;

                match strategy {
                    MergeStrategy::Sum | MergeStrategy::Average => entry.score += item.score,
                    MergeStrategy::Max => entry.score = entry.score.max(item.score),
                    MergeStrategy::Min => entry.score = if entry.score == 0.0 { item.score } else { entry.score.min(item.score) },
                    MergeStrategy::First => { /* Handled by initial insertion */ }
                }
            }
        }

        if strategy == MergeStrategy::Average {
            for (id, item) in &mut item_map {
                if let Some(&count) = counts.get(id) {
                    if count > 0 {
                        item.score /= count as f32;
                    }
                }
            }
        }

        let mut final_items: Vec<ScoredItem> = item_map.into_values().collect();
        final_items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        final_items
    }

    fn interleave_results(&self, pattern: &[String], mut sources: HashMap<String, Vec<ScoredItem>>) -> Vec<ScoredItem> {
        let mut result = Vec::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let total_slots = 100; // Cap at 100 items for interleaving

        for _ in 0..(total_slots / pattern.len() + 1) {
            for source_name in pattern {
                if let Some(items) = sources.get_mut(source_name) {
                    let idx = indices.entry(source_name.clone()).or_insert(0);
                    if *idx < items.len() {
                        result.push(items[*idx].clone());
                        *idx += 1;
                    }
                }
                if result.len() >= total_slots { break; }
            }
            if result.len() >= total_slots { break; }
        }
        
        result
    }

    fn evaluate_condition(&self, condition: &crate::pipeline::BranchCondition, context: &ExecutionContext) -> bool {
        let val = match condition.key.as_str() {
            "user_id" => context.user_id.map(|id| serde_json::json!(id)),
            "profile_id" => context.profile_id.as_ref().map(|id| serde_json::json!(id)),
            "maturity_rating" => context.maturity_rating.as_ref().map(|r| serde_json::json!(r)),
            "device_type" => context.device_type.as_ref().map(|t| serde_json::json!(t)),
            "location" => context.location.as_ref().map(|l| serde_json::json!(l)),
            _ => context.experiment_overrides.get(&condition.key).cloned(),
        };

        if let Some(val) = val {
            match condition.operator.as_str() {
                "==" | "eq" => val == condition.value,
                "!=" | "ne" => val != condition.value,
                _ => false
            }
        } else {
            false
        }
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
            Err(e) => Err(e),
        }
    }
}
