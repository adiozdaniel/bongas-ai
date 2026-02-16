use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Boost factor for trending items (e.g., 1.5 = 50% boost)
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Time window in hours to calculate trending score
    #[serde(default = "default_hours")]
    time_window_hours: i32,
    /// Minimum trending score to apply boost
    #[serde(default)]
    min_trending_score: f32,
}

fn default_boost() -> f32 {
    1.5
}

fn default_hours() -> i32 {
    24
}

pub struct BoostTrendingStage;

#[async_trait]
impl PipelineStage for BoostTrendingStage {
    fn name(&self) -> &str {
        "boost_trending"
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        // Query ClickHouse for trending scores
        let query = format!(
            r#"
            SELECT
                video_id,
                count() as view_count,
                uniqExact(user_id) as unique_viewers,
                avg(watch_percentage) as avg_completion
            FROM playback_sessions
            WHERE event_time >= now() - INTERVAL {} HOUR
                AND video_id IN ({})
            GROUP BY video_id
            "#,
            params.time_window_hours,
            item_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct TrendingRow {
            video_id: i32,
            view_count: u64,
            unique_viewers: u64,
            avg_completion: f32,
        }

        let client = _context.clickhouse_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ClickHouse client not configured"))?;

        let rows: Vec<TrendingRow> = client
            .query(&query)
            .fetch_all()
            .await?;

        // Calculate trending scores
        let trending_map: HashMap<i32, f32> = rows
            .into_iter()
            .map(|row| {
                let trending_score = (row.view_count as f32)
                    * (row.unique_viewers as f32 + 1.0).ln()
                    * row.avg_completion;
                (row.video_id, trending_score)
            })
            .collect();

        // Find max trending score for normalization
        let max_score = trending_map.values().cloned().fold(0.0f32, f32::max);

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some(&trending_score) = trending_map.get(&item.item_id) {
                    if trending_score >= params.min_trending_score && max_score > 0.0 {
                        let normalized_trending = trending_score / max_score;
                        item.score *= 1.0 + (params.boost_factor - 1.0) * normalized_trending;
                        item.metadata["trending_score"] = serde_json::json!(trending_score);
                        item.metadata["is_trending"] = serde_json::json!(true);
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
