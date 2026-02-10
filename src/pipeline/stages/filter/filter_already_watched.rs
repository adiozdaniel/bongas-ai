use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashSet;

pub struct FilterAlreadyWatchedStage;

#[async_trait]
impl PipelineStage for FilterAlreadyWatchedStage {
    fn name(&self) -> &str {
        "filter_already_watched"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT DISTINCT item_id
            FROM user_interactions
            WHERE user_id = $1
                AND interaction_type = 'view'
                AND completion_percentage > 0.9
            "#,
        )
        .bind(user_id)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let watched_ids: HashSet<i32> = rows.into_iter().map(|r| r.item_id).collect();

        let filtered: Vec<ScoredItem> = input.into_iter()
            .filter(|item| !watched_ids.contains(&item.item_id))
            .collect();

        Ok(filtered)
    }
}
