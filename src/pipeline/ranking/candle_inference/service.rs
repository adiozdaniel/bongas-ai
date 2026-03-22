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
    /// Model name for inference
    pub model_name: String,
    /// Model version (optional, defaults to latest)
    pub version: Option<String>,
    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Input dimension for user features
    #[serde(default = "default_user_dim")]
    pub user_dim: usize,
    /// Input dimension for item features
    #[serde(default = "default_item_dim")]
    pub item_dim: usize,
}

fn default_batch_size() -> usize { 64 }
fn default_user_dim() -> usize { 128 }
fn default_item_dim() -> usize { 128 }

pub struct CandleInferenceStage;

#[async_trait]
impl PipelineStage for CandleInferenceStage {
    fn name(&self) -> &str {
        "candle_inference"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse candle_inference params")?;

        if input.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            model = %params.model_name,
            version = ?params.version,
            item_count = input.len(),
            "Running Candle batch inference"
        );

        // 1. Get Model
        let model: Arc<CandleInferenceEngine> = match &params.version {
            Some(version) => {
                context.model_loader
                    .get_model_version(&params.model_name, version)
                    .await?
            },
            None => {
                context.model_loader
                    .get_model(&params.model_name)
                    .await?
            }
        };

        let start = std::time::Instant::now();
        let mut results = input.clone();

        // 2. Resolve User Features (Cached in context)
        let profile_id = context.profile_id.as_deref().unwrap_or("adult_default");
        let user_features = context.feature_store.get_profile_features(profile_id, params.user_dim).await?;

        // 3. Batch Inference Loop
        for chunk_slice in input.chunks(params.batch_size) {
            let chunk = chunk_slice.to_vec();
            let item_ids: Vec<i32> = chunk.iter().map(|i| i.item_id).collect();
            
            // Resolve Item Features
            let item_features_map = context.feature_store.get_item_features(&item_ids, params.item_dim).await?;

            for id in &item_ids {
                let user_batch = user_features.clone();
                let item_batch = item_features_map.get(id).cloned().unwrap_or_else(|| vec![0.0; params.item_dim]);

                // Execute Inference
                let score = model.clone().predict_batch(user_batch, item_batch).await
                    .context("Batch inference failed in candle_inference stage")? [0];

                // Update scores
                if let Some(res_item) = results.iter_mut().find(|r| r.item_id == *id) {
                    res_item.score = score;
                    res_item.metadata["inference_score"] = json!(score);
                }
            }
        }

        info!(
            request_id = %context.request_id,
            duration_ms = start.elapsed().as_millis(),
            "Candle batch inference completed"
        );

        Ok(results)
    }
}
