use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::analytics::clickhouse::ClickHouseClient;

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct WatchProgressItem {
    pub user_id: u32,
    pub item_id: u32,
    pub completion_rate: f32,
    pub watch_duration_seconds: u32,
    pub total_duration_seconds: u32,
    pub last_watched_at: u32, // Unix timestamp from ClickHouse DateTime
}

pub struct WatchProgressQuery {
    client: Arc<ClickHouseClient>,
}

impl WatchProgressQuery {
    pub fn new(client: Arc<ClickHouseClient>) -> Self {
        Self { client }
    }

    /// Fetch items user started but didn't finish (10-90% completion)
    pub async fn get_continue_watching(
        &self,
        user_id: u32,
        min_completion: f32,
        max_completion: f32,
        limit: usize,
    ) -> Result<Vec<WatchProgressItem>> {
        let query = format!(
            r#"
            SELECT
                user_id,
                item_id,
                completion_rate,
                watch_duration_seconds,
                total_duration_seconds,
                toUnixTimestamp(max(last_position_at)) as last_watched_at
            FROM playback_sessions
            WHERE user_id = {}
              AND completion_rate >= {}
              AND completion_rate <= {}
              AND completed = 0
              AND started_at > now() - INTERVAL 30 DAY
            GROUP BY user_id, item_id, completion_rate, watch_duration_seconds, total_duration_seconds
            ORDER BY last_watched_at DESC
            LIMIT {}
            "#,
            user_id, min_completion, max_completion, limit
        );

        let rows = self.client.inner().query(&query).fetch_all().await?;
        Ok(rows)
    }

    /// Get user's recently watched item IDs
    pub async fn get_recent_watches(
        &self,
        user_id: u32,
        days: u32,
        limit: usize,
    ) -> Result<Vec<u32>> {
        let query = format!(
            r#"
            SELECT DISTINCT item_id
            FROM playback_sessions
            WHERE user_id = {}
              AND started_at > now() - INTERVAL {} DAY
            ORDER BY started_at DESC
            LIMIT {}
            "#,
            user_id, days, limit
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct ItemIdRow {
            item_id: u32,
        }

        let rows: Vec<ItemIdRow> = self.client.inner().query(&query).fetch_all().await?;
        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Get user's completed item IDs (for filtering)
    pub async fn get_completed_items(&self, user_id: u32) -> Result<Vec<u32>> {
        let query = format!(
            r#"
            SELECT DISTINCT item_id
            FROM playback_sessions
            WHERE user_id = {}
              AND completed = 1
            "#,
            user_id
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct ItemIdRow {
            item_id: u32,
        }

        let rows: Vec<ItemIdRow> = self.client.inner().query(&query).fetch_all().await?;
        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }
}
