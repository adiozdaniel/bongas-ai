use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Boost factor for high completion rate content
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Minimum completion rate to apply boost (0.0-1.0)
    #[serde(default = "default_min_rate")]
    min_completion_rate: f32,
    /// Minimum number of views to consider completion rate reliable
    #[serde(default = "default_min_views")]
    min_views: i32,
    /// Time window in days to calculate completion rate
    #[serde(default = "default_days")]
    time_window_days: i32,
}

fn default_boost() -> f32 {
    1.4
}

fn default_min_rate() -> f32 {
    0.7
}

fn default_min_views() -> i32 {
    10
}

fn default_days() -> i32 {
    30
}

pub struct BoostCompletionRateStage;

#[async_trait]
impl PipelineStage for BoostCompletionRateStage {
    fn name(&self) -> &str {
        "boost_completion_rate"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        // Query ClickHouse for completion rates
        let query = format!(
            r#"
            SELECT
                video_id,
                avg(watch_percentage) as avg_completion,
                count() as total_views,
                countIf(watch_percentage >= 0.9) as completed_views
            FROM playback_sessions
            WHERE event_time >= now() - INTERVAL {} DAY
                AND video_id IN ({})
            GROUP BY video_id
            HAVING total_views >= {}
            "#,
            params.time_window_days,
            item_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
            params.min_views
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct CompletionRow {
            video_id: i32,
            avg_completion: f32,
            total_views: u64,
            completed_views: u64,
        }

        let rows: Vec<CompletionRow> = context.clickhouse
            .inner()
            .query(&query)
            .fetch_all()
            .await
            .unwrap_or_default();

        let completion_map: HashMap<i32, (f32, u64, u64)> = rows
            .into_iter()
            .map(|row| (row.video_id, (row.avg_completion, row.total_views, row.completed_views)))
            .collect();

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some((avg_completion, total_views, completed_views)) = completion_map.get(&item.item_id) {
                    if *avg_completion >= params.min_completion_rate {
                        // Boost proportional to how much above minimum
                        let excess = avg_completion - params.min_completion_rate;
                        let max_excess = 1.0 - params.min_completion_rate;
                        let normalized = if max_excess > 0.0 { excess / max_excess } else { 0.0 };

                        let boost = 1.0 + (params.boost_factor - 1.0) * normalized;
                        item.score *= boost;

                        item.metadata["completion_rate"] = serde_json::json!(avg_completion);
                        item.metadata["completion_stats"] = serde_json::json!({
                            "total_views": total_views,
                            "completed_views": completed_views,
                            "completion_percentage": (completed_views * 100) / total_views.max(&1),
                        });
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
