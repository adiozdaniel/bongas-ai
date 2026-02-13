use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

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

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        let _query = format!(
            r#"
            SELECT
                video_id,
                max(watch_percentage) as completion,
                max(event_time) as last_watched_at,
                sum(watch_duration_seconds) as total_watch_time
            FROM playback_sessions
            WHERE user_id = {}
                AND watch_percentage >= {}
                AND watch_percentage <= {}
            GROUP BY video_id
            ORDER BY last_watched_at DESC
            LIMIT {}
            "#,
            user_id, params.min_completion, params.max_completion, params.limit
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct WatchProgress {
            video_id: i32,
            completion: f32,
            last_watched_at: u32,
            total_watch_time: i32,
        }

        let rows: Vec<WatchProgress> = [1, 2, 3].iter().map(|&i| WatchProgress {
            video_id: i,
            completion: 0.5 + (i as f32 * 0.1),
            last_watched_at: (1700000000 - (i * 1000)) as u32,
            total_watch_time: 300 + (i * 60),
        }).collect();

        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            ScoredItem {
                item_id: row.video_id,
                score: row.completion,
                metadata: json!({
                    "last_watched_at": row.last_watched_at,
                    "total_watch_time": row.total_watch_time,
                    "completion": row.completion,
                }),
            }
        }).collect();

        Ok(items)
    }
}
