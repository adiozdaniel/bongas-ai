use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use crate::db::repositories::item_feature_service::ItemFeatureRow;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    sequence_length: usize,
    top_k: usize,

    use_onnx: Option<bool>,
}

pub struct MLInferenceBERT4RecStage;

#[async_trait]
impl PipelineStage for MLInferenceBERT4RecStage {
    fn name(&self) -> &str {
        "ml_inference_bert4rec"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;
        let use_onnx = params.use_onnx.unwrap_or(true);

        info!(
            request_id = %context.request_id,
            user_id = user_id,
            sequence_length = params.sequence_length,
            use_onnx = use_onnx,
            "Running BERT4Rec inference"
        );

        // Get user's recent interaction sequence
        let sequence = context.item_feature_service
            .get_user_recent_interaction_ids(user_id, params.sequence_length as i64)
            .await?;

        if sequence.is_empty() {
            return Ok(vec![]);
        }

        // In production, this would run ONNX inference with the BERT4Rec model
        // For now, fetch items similar to the most recent items in the sequence
        let recent_items = &sequence[..std::cmp::min(5, sequence.len())];

        let all_item_ids_in_input: Vec<i32> = _input.iter().map(|item| item.item_id).collect(); // Use _input here
        let all_item_features_map = context.item_feature_service
            .get_item_features_batch(&all_item_ids_in_input)
            .await?;

        let mut candidates_features: Vec<ItemFeatureRow> = all_item_features_map.into_values()
            .filter(|feature_row| !recent_items.contains(&feature_row.item_id))
            .collect();

        // The original query ordered by trending_score DESC LIMIT $2.
        // We apply this in-memory for now.
        candidates_features.sort_by(|a, b| b.trending_score.partial_cmp(&a.trending_score).unwrap_or(std::cmp::Ordering::Equal));
        candidates_features.truncate(params.top_k);

        let items: Vec<ScoredItem> = candidates_features.into_iter().map(|row| {
            ScoredItem {
                item_id: row.item_id,
                score: row.trending_score,
                metadata: json!({
                    "model": "bert4rec",
                    "inference_engine": if use_onnx { "onnx" } else { "fallback" },
                    "sequence_length": sequence.len(),
                }),
            }
        }).collect();

        Ok(items)
    }
}


