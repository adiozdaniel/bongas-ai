use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;


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

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features_map = context.item_feature_service
            .get_item_features_batch(item_ids.as_slice())
            .await?;

        let pop_map: HashMap<i32, f32> = item_features_map
            .into_iter()
            .map(|(item_id, features)| (item_id, features.trending_score))
            .collect();

        let boosted: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            if let Some(&pop_score) = pop_map.get(&item.item_id) {
                item.score = item.score * (1.0 - params.weight) + pop_score * params.weight;
            }
            item
        }).collect();

        Ok(boosted)
    }
}
