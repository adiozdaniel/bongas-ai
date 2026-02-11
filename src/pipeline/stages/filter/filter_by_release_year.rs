use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Minimum release year (optional)
    #[serde(default)]
    min_year: Option<i32>,
    /// Maximum release year (optional)
    #[serde(default)]
    max_year: Option<i32>,
    /// Specific years to include (optional, overrides min/max)
    #[serde(default)]
    years: Option<Vec<i32>>,
    /// Whether to include items with unknown release year
    #[serde(default)]
    include_unknown: bool,
}

pub struct FilterByReleaseYearStage;

#[async_trait]
impl PipelineStage for FilterByReleaseYearStage {
    fn name(&self) -> &str {
        "filter_by_release_year"
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
            release_year: Option<i32>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, release_year
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let year_map: HashMap<i32, Option<i32>> = rows
            .into_iter()
            .map(|row| (row.item_id, row.release_year))
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match year_map.get(&item.item_id) {
                    Some(Some(year)) => {
                        // If specific years are provided, check against those
                        if let Some(ref years) = params.years {
                            return years.contains(year);
                        }
                        // Otherwise check min/max range
                        let above_min = params.min_year.map_or(true, |min| *year >= min);
                        let below_max = params.max_year.map_or(true, |max| *year <= max);
                        above_min && below_max
                    }
                    _ => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
