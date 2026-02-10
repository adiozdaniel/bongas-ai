use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    #[allow(dead_code)]
    model_name: String,
    algorithm: String,
    explore_rate: f32,
    #[allow(dead_code)]
    context_features: Option<Vec<String>>,
}

pub struct ONNXInferenceBanditStage;

#[async_trait]
impl PipelineStage for ONNXInferenceBanditStage {
    fn name(&self) -> &str {
        "onnx_inference_bandit"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let _user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        info!(
            request_id = %context.request_id,
            algorithm = %params.algorithm,
            explore_rate = params.explore_rate,
            input_count = input.len(),
            "Running ONNX bandit inference"
        );

        // Apply exploration/exploitation using the configured algorithm
        let updated_items: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            // Thompson sampling or LinUCB score adjustment
            let exploration_bonus = match params.algorithm.as_str() {
                "thompson_sampling" => {
                    // Simplified Thompson sampling: add random noise proportional to explore_rate
                    let noise = (rand_score(item.item_id) - 0.5) * 2.0 * params.explore_rate;
                    noise
                }
                "linucb" => {
                    // Simplified LinUCB: add uncertainty bonus
                    params.explore_rate * (1.0 / (item.score.abs() + 1.0)).sqrt()
                }
                _ => 0.0,
            };

            item.score += exploration_bonus;
            item.metadata["bandit_algorithm"] = json!(params.algorithm);
            item.metadata["explore_rate"] = json!(params.explore_rate);
            item.metadata["exploration_bonus"] = json!(exploration_bonus);
            item
        }).collect();

        Ok(updated_items)
    }
}

/// Deterministic pseudo-random score based on item_id (no external RNG dependency)
fn rand_score(item_id: i32) -> f32 {
    let hash = ((item_id as u64).wrapping_mul(2654435761)) % 1000;
    hash as f32 / 1000.0
}
