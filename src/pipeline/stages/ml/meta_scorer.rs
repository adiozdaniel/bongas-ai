use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use tracing::warn;

#[derive(Deserialize)]
struct Params {
    /// ONNX model name for the meta-scorer (e.g., "meta_ranker_v1")
    model_name: String,
    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    batch_size: usize,
}

fn default_batch_size() -> usize { 64 }

/// "The ML Sophistication Bridge": Uses an ONNX model to combine multiple heuristics.
pub struct MetaScorerStage;

#[async_trait]
impl PipelineStage for MetaScorerStage {
    fn name(&self) -> &str {
        "meta_scorer"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
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

        // 1. Get model
        let model = context.model_loader
            .get_model(&params.model_name)
            .await
            .with_context(|| format!("Failed to load meta-scorer model: {}", params.model_name))?;

        // 2. Prepare features (batch items)
        let mut feature_batch: Vec<Vec<f32>> = Vec::with_capacity(input.len());
        for item in &input {
            let vector = item.metadata.get("heuristic_vector")
                .and_then(|v| serde_json::from_value::<Vec<f32>>(v.clone()).ok())
                .unwrap_or_default();
            feature_batch.push(vector);
        }

        if feature_batch.is_empty() || feature_batch[0].is_empty() {
            warn!("Meta-scorer skipped: No heuristic_vector found in metadata. Run heuristic_aggregator first.");
            return Ok(input);
        }

        // 3. Run Inference
        let mut final_scores: Vec<f32> = Vec::with_capacity(input.len());
        {
            let mut engine = model.write().await;
            
            for chunk_idx in (0..feature_batch.len()).step_by(params.batch_size) {
                let end = std::cmp::min(chunk_idx + params.batch_size, feature_batch.len());
                let chunk = feature_batch[chunk_idx..end].to_vec();
                
                // Meta-scorer uses a simplified model (usually 1 input: the feature vector)
                // We adapt predict_batch or use a generic inference method if available.
                // Assuming our ONNX Engine supports single-input batching.
                let chunk_scores = engine.predict_batch(chunk.clone(), chunk) // Mocking dual input for now if engine requires it
                    .context("Meta-scorer ONNX inference failed")?;
                
                final_scores.extend(chunk_scores);
            }
        }

        // 4. Update items
        let processed: Vec<ScoredItem> = input.into_iter().zip(final_scores.into_iter())
            .map(|(mut item, ml_score)| {
                item.score = ml_score;
                item.metadata["ml_ranked"] = json!(true);
                item
            }).collect();

        Ok(processed)
    }
}
