use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

pub struct EnrichTimeRemainingStage;

#[async_trait]
impl PipelineStage for EnrichTimeRemainingStage {
    fn name(&self) -> &str {
        "enrich_time_remaining"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
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

        let duration_map: HashMap<i32, i32> = rows
            .into_iter()
            .filter_map(|row| row.duration_seconds.map(|d| (row.item_id, d)))
            .collect();

        let enriched: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            if let Some(&duration) = duration_map.get(&item.item_id) {
                if let Some(completion) = item.metadata.get("completion").and_then(|v| v.as_f64()) {
                    let watched_seconds = (duration as f64 * completion) as i32;
                    let remaining_seconds = duration - watched_seconds;

                    item.metadata["duration_seconds"] = json!(duration);
                    item.metadata["watched_seconds"] = json!(watched_seconds);
                    item.metadata["remaining_seconds"] = json!(remaining_seconds);
                }
            }
            item
        }).collect();

        Ok(enriched)
    }
}
