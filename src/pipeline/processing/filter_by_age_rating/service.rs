use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, MaturityRating};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Maximum allowed age rating (e.g., "G", "PG", "PG-13", "R", "NC-17")
    max_rating: String,
    /// Whether to include items with unknown ratings
    #[serde(default = "default_include_unknown")]
    include_unknown: bool,
}

fn default_include_unknown() -> bool {
    false
}

pub struct FilterByAgeRatingStage;

#[async_trait]
impl PipelineStage for FilterByAgeRatingStage {
    fn name(&self) -> &str {
        "filter_by_age_rating"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let max_rating = MaturityRating::from_str(&params.max_rating);
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features_map = context.item_feature_service
            .get_item_features_batch(item_ids.as_slice())
            .await?;

        let rating_map: HashMap<i32, Option<String>> = item_features_map
            .into_iter()
            .map(|(item_id, features)| (item_id, features.age_rating))
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match rating_map.get(&item.item_id) {
                    Some(Some(rating)) => MaturityRating::from_str(rating) <= max_rating,
                    Some(None) | None => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
