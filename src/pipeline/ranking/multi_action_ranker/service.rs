use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use std::sync::Arc;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use crate::ml::inference::candle::service::CandleInferenceEngine;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    /// Model name for scoring
    pub model_name: String,
    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Weights for different actions (e.g. click: 1.0, watch: 2.0)
    pub _action_weights: HashMap<String, f32>,
}

fn default_batch_size() -> usize { 64 }

use std::collections::HashMap;

pub struct MultiActionRankerStage;

#[async_trait]
impl PipelineStage for MultiActionRankerStage {
    fn name(&self) -> &str {
        "multi_action_ranker"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse multi_action_ranker params")?;

        if input.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            model = %params.model_name,
            item_count = input.len(),
            "Running multi-action inference"
        );

        let model: Arc<CandleInferenceEngine> = context.model_loader
            .get_model(&params.model_name)
            .await?;

        let start = std::time::Instant::now();
        let mut results = input.clone();

        for chunk_slice in input.chunks(params.batch_size) {
            let chunk = chunk_slice.to_vec();
            let _item_ids: Vec<i32> = chunk.iter().map(|i| i.item_id).collect();
            
            // Mock fetching features - in real impl use context.feature_store
            let user_features = vec![0.0; 128];
            let item_features = vec![0.0; 128];

            // Inference
            let chunk_probs: Vec<Vec<f32>> = model.clone().predict_multi_action(user_features, item_features).await
                .context("Multi-action inference failed")?;

            // Compute expected value based on weights
            for (i, probs) in chunk_probs.into_iter().enumerate() {
                let mut expected_value = 0.0;
                // Probs correspond to model output order (e.g. 0: click, 1: watch)
                // In real impl, map indices to action names from params.action_weights
                for prob in probs.iter() {
                    expected_value += prob * 1.0; // Placeholder weight
                }

                if let Some(chunk_item) = chunk.get(i) {
                    if let Some(global_idx) = results.iter().position(|r| r.item_id == chunk_item.item_id) {
                        results[global_idx].score = expected_value;
                        results[global_idx].metadata["multi_action_probs"] = json!(probs);
                    }
                }
            }
        }

        info!(
            request_id = %context.request_id,
            duration_ms = start.elapsed().as_millis(),
            "Multi-action ranking completed"
        );

        Ok(results)
    }
}
