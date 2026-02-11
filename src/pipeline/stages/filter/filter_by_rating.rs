use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

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

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            user_rating: Option<f32>,
            user_rating_count: Option<i32>,
            critic_rating: Option<f32>,
            critic_rating_count: Option<i32>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, user_rating, user_rating_count, critic_rating, critic_rating_count
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let rating_map: HashMap<i32, (Option<f32>, Option<i32>)> = rows
            .into_iter()
            .map(|row| {
                let (rating, count) = match params.source.as_str() {
                    "user" => (row.user_rating, row.user_rating_count),
                    "critic" => (row.critic_rating, row.critic_rating_count),
                    _ => {
                        // Combined: weighted average if both exist
                        let combined_rating = match (row.user_rating, row.critic_rating) {
                            (Some(u), Some(c)) => Some((u * 0.7 + c * 0.3)),
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
                (row.item_id, (rating, count))
            })
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match rating_map.get(&item.item_id) {
                    Some((Some(rating), count)) => {
                        // Check rating count threshold
                        if let Some(min_count) = params.min_rating_count {
                            if count.unwrap_or(0) < min_count {
                                return params.include_unrated;
                            }
                        }
                        // Check rating range
                        let above_min = params.min_rating.map_or(true, |min| *rating >= min);
                        let below_max = params.max_rating.map_or(true, |max| *rating <= max);
                        above_min && below_max
                    }
                    _ => params.include_unrated,
                }
            })
            .collect();

        Ok(filtered)
    }
}
