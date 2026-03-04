use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use crate::ml::assets::utils::service::apply_weights_simd;

#[derive(Deserialize)]
struct Params {
    weight: f32,
}

pub struct BoostByPopularityStage;

#[async_trait]
impl PipelineStage for BoostByPopularityStage {
    fn name(&self) -> &str {
        "boost_by_popularity"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn is_fusable(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        if input.is_empty() { return Ok(input); }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = context.item_feature_service
            .get_item_features_batch(item_ids.as_slice())
            .await?;

        // 1. Prepare vectors for SIMD
        let mut current_scores: Vec<f32> = input.iter().map(|i| i.score).collect();
        let boosts: Vec<f32> = input.iter().map(|item| {
            item_features_map.get(&item.item_id)
                .map(|f| f.trending_score)
                .unwrap_or(0.0)
        }).collect();

        // 2. Hardware-level vectorization
        apply_weights_simd(&mut current_scores, &boosts, params.weight);

        // 3. Re-inject scores
        let boosted: Vec<ScoredItem> = input.into_iter().zip(current_scores.into_iter())
            .map(|(mut item, new_score)| {
                item.score = new_score;
                item
            }).collect();

        Ok(boosted)
    }
}
