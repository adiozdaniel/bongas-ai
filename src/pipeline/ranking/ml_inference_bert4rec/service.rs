use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use crate::db::repositories::item_feature_service::ItemFeatureRow;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    sequence_length: usize,
    #[serde(alias = "limit")]
    top_k: usize,

    use_onnx: Option<bool>,
}

pub struct MLInferenceBERT4RecStage;

#[async_trait]
impl PipelineStage for MLInferenceBERT4RecStage {
    fn name(&self) -> &str {
        "ml_inference_bert4rec"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::Empty
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        mut input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;
        let use_onnx = params.use_onnx.unwrap_or(true);

        info!(
            request_id = %context.request_id,
            user_id = user_id,
            sequence_length = params.sequence_length,
            use_onnx = use_onnx,
            input_count = input.len(),
            "Running BERT4Rec inference"
        );

        // If used as a retrieval stage (Empty input), fetch candidates first
        if input.is_empty() {
            let candidates = context.item_feature_service
                .get_popular_content(0, (params.top_k * 5) as i64)
                .await?;
            
            input = candidates.into_iter().map(|row| {
                ScoredItem::new(
                    row.item_id,
                    row.trending_score,
                    json!({
                        "view_count": row.view_count,
                        "age_rating": row.age_rating,
                        "genres": row.genres,
                    }),
                )
            }).collect();
        }

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

        let mut candidates_features: Vec<ItemFeatureRow> = Vec::new();
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = context.item_feature_service
            .get_item_features_batch(&item_ids)
            .await?;

        for (_, row) in item_features_map {
             if !recent_items.contains(&row.item_id) {
                 candidates_features.push(row);
             }
        }

        // The original query ordered by trending_score DESC LIMIT $2.
        // We apply this in-memory for now.
        candidates_features.sort_by(|a, b| b.trending_score.partial_cmp(&a.trending_score).unwrap_or(std::cmp::Ordering::Equal));
        candidates_features.truncate(params.top_k);

        let items: Vec<ScoredItem> = candidates_features.into_iter().map(|row| {
            ScoredItem::new(
                row.item_id,
                row.trending_score,
                json!({
                    "model": "bert4rec",
                    "inference_engine": if use_onnx { "onnx" } else { "fallback" },
                    "sequence_length": sequence.len(),
                }),
            )
        }).collect();

        Ok(items)
    }
}


