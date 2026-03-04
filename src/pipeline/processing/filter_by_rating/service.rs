use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Minimum user/critic rating (0.0 - 10.0 scale)
    #[serde(default)]
    min_rating: Option<f32>,
    /// Maximum rating (optional)
    #[serde(default)]
    max_rating: Option<f32>,
    /// Minimum number of ratings required
    #[serde(default)]
    min_rating_count: Option<i32>,
    /// Rating source: "user", "critic", "combined" (default)
    #[serde(default = "default_source")]
    source: String,
    /// Whether to include items with no ratings
    #[serde(default)]
    include_unrated: bool,
}

fn default_source() -> String {
    "combined".to_string()
}

pub struct FilterByRatingStage;

#[async_trait]
impl PipelineStage for FilterByRatingStage {
    fn name(&self) -> &str {
        "filter_by_rating"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        let (rating, count) = match params.source.as_str() {
                            "user" => (row.user_rating, row.user_rating_count),
                            "critic" => (row.critic_rating, row.critic_rating_count),
                            _ => {
                                // Combined: weighted average if both exist
                                let combined_rating = match (row.user_rating, row.critic_rating) {
                                    (Some(u), Some(c)) => Some(u * 0.7 + c * 0.3 ),
                                    (Some(u), None) => Some(u),
                                    (None, Some(c)) => Some(c),
                                    (None, None) => None,
                                };
                                let combined_count = match (row.user_rating_count, row.critic_rating_count) {
                                    (Some(u), Some(c)) => Some(u + c),
                                    (Some(u), None) => Some(u),
                                    (None, Some(c)) => Some(c),
                                    (None, None) => None,
                                };
                                (combined_rating, combined_count)
                            }
                        };

                        if let Some(r) = rating {
                            // Check rating count threshold
                            if let Some(min_count) = params.min_rating_count {
                                if count.unwrap_or(0) < min_count {
                                    return params.include_unrated;
                                }
                            }
                            // Check rating range
                            let above_min = params.min_rating.map_or(true, |min| r >= min);
                            let below_max = params.max_rating.map_or(true, |max| r <= max);
                            above_min && below_max
                        } else {
                            params.include_unrated
                        }
                    }
                    None => params.include_unrated,
                }
            })
            .collect();

        Ok(filtered)
    }
}
