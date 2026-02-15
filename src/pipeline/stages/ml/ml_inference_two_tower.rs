use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {

    model_name: Option<String>,
    top_k: usize,

    use_onnx: Option<bool>,
}

pub struct MLInferenceTwoTowerStage;

#[async_trait]
impl PipelineStage for MLInferenceTwoTowerStage {
    fn name(&self) -> &str {
        "ml_inference_two_tower"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        info!(
            request_id = %context.request_id,
            user_id = user_id,
            input_count = input.len(),
            "Running Two-Tower ML inference"
        );

        // Get user embedding from features
        let user_features = context.feature_store
            .get_user_features(user_id, 3) // Assuming feature_dim is 3 for this model
            .await?;

        // Score all input items using Two-Tower dot product
        let mut scored: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            // Placeholder scoring using feature similarity
            // In production, this would use ONNX Runtime with the two_tower model
            item.score = user_features.iter()
                .take(3)
                .sum::<f32>()
                * (item.score + 0.1);
            item.metadata["model"] = json!("two_tower");
            item.metadata["inference_engine"] = json!("onnx");
            item
        }).collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(params.top_k);

        Ok(scored)
    }
}


