use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use std::sync::Arc;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use crate::ml::inference::onnx::service::OnnxInferenceEngine;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    /// Model name for scoring
    pub model_name: String,
    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_batch_size() -> usize { 64 }

pub struct MetaScorerStage;

#[async_trait]
impl PipelineStage for MetaScorerStage {
    fn name(&self) -> &str {
        "meta_scorer"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse meta_scorer params")?;

        if input.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            model = %params.model_name,
            item_count = input.len(),
            "Running meta-scorer inference"
        );

        let model: Arc<OnnxInferenceEngine> = context.model_loader
            .get_model(&params.model_name)
            .await?;

        let start = std::time::Instant::now();
        let mut results = input.clone();

        // ─── Batch Inference Loop ───────────────────────────────────────────
        // We chunk the input to respect model batch limits and prevent bulkhead saturation
        for chunk_slice in input.chunks(params.batch_size) {
            let chunk = chunk_slice.to_vec();
            
            // Get features for this batch
            let item_ids: Vec<i32> = chunk.iter().map(|i| i.item_id).collect();
            let item_features = context.feature_store.get_item_features(&item_ids, 128).await?;
            let user_features = context.feature_store.get_user_features(context.user_id.unwrap_or(0), 128).await?;

            // Prepare tensors
            let mut user_batch = Vec::with_capacity(chunk.len());
            let mut item_batch = Vec::with_capacity(chunk.len());

            for id in item_ids {
                user_batch.push(user_features.clone());
                item_batch.push(item_features.get(&id).cloned().unwrap_or_else(|| vec![0.0; 128]));
            }

            // Inference
            let chunk_scores: Vec<f32> = model.clone().predict_batch(user_batch, item_batch).await
                .context("Batch inference failed in meta_scorer")?;

            // Update scores in result set
            for (i, score) in chunk_scores.into_iter().enumerate() {
                let global_idx = results.iter().position(|r| r.item_id == chunk[i].item_id).unwrap();
                results[global_idx].score = score;
                results[global_idx].metadata["meta_score"] = json!(score);
            }
        }

        let duration = start.elapsed();
        info!(
            request_id = %context.request_id,
            duration_ms = duration.as_millis(),
            "Meta-scorer completed"
        );

        Ok(results)
    }
}
