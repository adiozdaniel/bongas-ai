use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use redis::AsyncCommands;

#[derive(Deserialize)]
struct Params {
    /// Time window in hours to aggregate interactions (default: 24)
    #[serde(default = "default_window")]
    time_window_hours: i32,
    /// Maximum items to recover
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_window() -> i32 { 24 }
fn default_limit() -> usize { 50 }

/// Recovery stage that surfaces content popular among behavioral lookalikes.
pub struct FetchBehavioralTribeStage;

#[async_trait]
impl PipelineStage for FetchBehavioralTribeStage {
    fn name(&self) -> &str {
        "fetch_behavioral_tribe"
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
        
        let profile_id = match &context.profile_id {
            Some(pid) => pid,
            None => return Ok(Vec::new()), // Anonymous users don't have tribes yet
        };

        // 1. Retrieve Tribe ID from Redis
        let tribe_id = if let Some(mut conn) = context.cache_manager.l2_connection() {
            let key = format!("tribe_map:{}", profile_id);
            conn.get::<_, Option<i32>>(key).await.unwrap_or(None)
        } else {
            None
        };

        let tribe_id = match tribe_id {
            Some(id) => id,
            None => return Ok(Vec::new()), // Fallback to other recovery stages
        };

        // 2. Query ClickHouse for Tribal Activity
        let query = r#"
            SELECT
                item_id,
                count() as popularity,
                avg(watch_duration_seconds) as avg_duration
            FROM user_interactions
            WHERE tribe_id = ?
              AND interaction_type IN ('playback_start', 'complete', 'click')
              AND created_at >= now() - INTERVAL ? HOUR
            GROUP BY item_id
            ORDER BY popularity DESC
            LIMIT ?
        "#;

        #[derive(clickhouse::Row, Deserialize)]
        struct TribalItem {
            item_id: i32,
            popularity: u64,
            avg_duration: f32,
        }

        let client = context.clickhouse_client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ClickHouse client not configured for Behavioral Tribes"))?;

        let rows: Vec<TribalItem> = client
            .query(query)
            .bind(tribe_id)
            .bind(params.time_window_hours)
            .bind(params.limit)
            .fetch_all()
            .await?;

        // 3. Convert to ScoredItems
        let items: Vec<ScoredItem> = rows.into_iter().map(|row| {
            // Normalized score calculation
            let score = (row.popularity as f32).log10() + 0.5; 

            ScoredItem::new(
                row.item_id,
                score,
                json!({
                    "source": "behavioral_tribe",
                    "tribe_id": tribe_id,
                    "popularity": row.popularity,
                    "avg_duration": row.avg_duration,
                }),
            )
        }).collect();

        Ok(items)
    }
}
