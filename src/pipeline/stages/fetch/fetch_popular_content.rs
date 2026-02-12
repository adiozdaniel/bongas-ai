use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use crate::analytics::AnalyticsManager;

#[derive(Deserialize)]
struct Params {
    limit: usize,
    min_views: Option<i32>,
}

pub struct FetchPopularContentStage;

#[async_trait]
impl PipelineStage for FetchPopularContentStage {
    fn name(&self) -> &str {
        "fetch_popular_content"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let min_views = params.min_views.unwrap_or(0);

        // Record ClickHouse query
        if let Some(analytics) = context.analytics() {
            analytics.record_clickhouse_query("fetch_popular_content", "item_features");
        }

        // Start timing the query
        let _timer = if let Some(analytics) = context.analytics() {
            Some(analytics.start_clickhouse_query_timer("fetch_popular_content"))
        } else {
            None
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            view_count: i32,
            trending_score: f32,
            completion_rate: f32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, view_count, trending_score, completion_rate
            FROM item_features
            WHERE view_count >= $1
            ORDER BY view_count DESC
            LIMIT $2
            "#,
        )
        .bind(min_views)
        .bind(params.limit as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            ScoredItem {
                item_id: row.item_id,
                score: row.trending_score,
                metadata: json!({
                    "view_count": row.view_count,
                    "completion_rate": row.completion_rate,
                }),
            }
        }).collect();

        Ok(items)
    }
}
