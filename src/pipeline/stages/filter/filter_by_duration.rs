use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Minimum duration in minutes (optional)
    #[serde(default)]
    min_minutes: Option<i32>,
    /// Maximum duration in minutes (optional)
    #[serde(default)]
    max_minutes: Option<i32>,
    /// Whether to include items with unknown duration
    #[serde(default)]
    include_unknown: bool,
}

pub struct FilterByDurationStage;

#[async_trait]
impl PipelineStage for FilterByDurationStage {
    fn name(&self) -> &str {
        "filter_by_duration"
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
            duration_seconds: Option<i32>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, duration_seconds
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let duration_map: HashMap<i32, Option<i32>> = rows
            .into_iter()
            .map(|row| (row.item_id, row.duration_seconds))
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match duration_map.get(&item.item_id) {
                    Some(Some(duration_seconds)) => {
                        let duration_minutes = duration_seconds / 60;
                        let above_min = params.min_minutes.map_or(true, |min| duration_minutes >= min);
                        let below_max = params.max_minutes.map_or(true, |max| duration_minutes <= max);
                        above_min && below_max
                    }
                    _ => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
