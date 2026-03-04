use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    min_completion: f32,
    max_completion: f32,
    limit: usize,
}

pub struct FetchClickHouseWatchProgressStage;

#[async_trait]
impl PipelineStage for FetchClickHouseWatchProgressStage {
    fn name(&self) -> &str {
        "fetch_clickhouse_watch_progress"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        let query = r#"
            SELECT
                video_id,
                max(watch_percentage) as completion,
                toUnixTimestamp(max(event_time)) as last_watched_at,
                sum(watch_duration_seconds) as total_watch_time
            FROM playback_sessions
            WHERE user_id = ?
                AND watch_percentage >= ?
                AND watch_percentage <= ?
            GROUP BY video_id
            ORDER BY last_watched_at DESC
            LIMIT ?
        "#;

        #[derive(clickhouse::Row, Deserialize)]
        struct WatchProgress {
            video_id: i32,
            completion: f32,
            last_watched_at: u32,
            total_watch_time: i32,
        }

        let client = context.clickhouse_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ClickHouse client not configured"))?;

        let rows: Vec<WatchProgress> = client
            .query(query)
            .bind(user_id)
            .bind(params.min_completion)
            .bind(params.max_completion)
            .bind(params.limit)
            .fetch_all()
            .await?;

        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            ScoredItem::new(
                row.video_id,
                row.completion,
                json!({
                    "last_watched_at": row.last_watched_at,
                    "total_watch_time": row.total_watch_time,
                    "completion": row.completion,
                }),
            )
        }).collect();

        Ok(items)
    }
}
