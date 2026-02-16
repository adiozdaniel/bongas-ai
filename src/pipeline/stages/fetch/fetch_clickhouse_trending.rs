use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    time_window_hours: i32,
    min_views: i32,
    limit: usize,
}

pub struct FetchClickHouseTrendingStage;

#[async_trait]
impl PipelineStage for FetchClickHouseTrendingStage {
    fn name(&self) -> &str {
        "fetch_clickhouse_trending"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Record ClickHouse query
        // if let Some(analytics) = _context.analytics() {
        //     analytics.record_clickhouse_query("fetch_clickhouse_trending", "playback_sessions");
        // }

        // Start timing the query
        // let _timer = if let Some(analytics) = _context.analytics() {
        //     Some(analytics.start_clickhouse_query_timer("fetch_clickhouse_trending"))
        // } else {
        //     None
        // };

        let query = format!(
            r#"
            SELECT
                video_id,
                count() as view_count,
                uniqExact(user_id) as unique_viewers,
                avg(watch_percentage) as avg_completion,
                count() / {} as views_per_hour
            FROM playback_sessions
            WHERE event_time >= now() - INTERVAL {} HOUR
            GROUP BY video_id
            HAVING view_count >= {}
            ORDER BY views_per_hour DESC, avg_completion DESC
            LIMIT {}
            "#,
            params.time_window_hours,
            params.time_window_hours,
            params.min_views,
            params.limit
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct TrendingItem {
            video_id: i32,
            view_count: u64,
            unique_viewers: u64,
            avg_completion: f32,
            views_per_hour: f32,
        }

        let client = _context.clickhouse_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ClickHouse client not configured"))?;

        let rows: Vec<TrendingItem> = client
            .query(&query)
            .fetch_all()
            .await?;

        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            let score = row.views_per_hour * row.avg_completion * (row.unique_viewers as f32 + 1.0).ln();

            ScoredItem::new(
                row.video_id,
                score,
                json!({
                    "view_count": row.view_count,
                    "unique_viewers": row.unique_viewers,
                    "avg_completion": row.avg_completion,
                    "views_per_hour": row.views_per_hour,
                }),
            )
        }).collect();

        Ok(items)
    }
}
