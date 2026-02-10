use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    limit: usize,
    hours: i32,
}

pub struct FetchRecentWatchesStage;

#[async_trait]
impl PipelineStage for FetchRecentWatchesStage {
    fn name(&self) -> &str {
        "fetch_recent_watches"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            last_watched: Option<chrono::DateTime<chrono::Utc>>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, MAX(created_at) as last_watched
            FROM user_interactions
            WHERE user_id = $1
                AND interaction_type = 'view'
                AND created_at >= NOW() - make_interval(hours => $2)
            GROUP BY item_id
            ORDER BY last_watched DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(params.hours)
        .bind(params.limit as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            ScoredItem {
                item_id: row.item_id,
                score: 1.0,
                metadata: json!({
                    "last_watched": row.last_watched.map(|t| t.to_rfc3339()),
                }),
            }
        }).collect();

        Ok(items)
    }
}
