use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::analytics::clickhouse::ClickHouseClient;

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct TrendingItem {
    pub item_id: u32,
    pub views_24h: u64,
    pub views_7d: u64,
    pub daily_avg: f64,
    pub velocity: f64,
    pub completion_rate_avg: f32,
}

pub struct TrendingQuery {
    client: Arc<ClickHouseClient>,
}

impl TrendingQuery {
    pub fn new(client: Arc<ClickHouseClient>) -> Self {
        Self { client }
    }

    /// Get trending items based on velocity (24h views vs 7-day average)
    pub async fn get_trending(
        &self,
        min_views: u64,
        min_velocity: f64,
        limit: usize,
    ) -> Result<Vec<TrendingItem>> {
        let query = format!(
            r#"
            SELECT
                item_id,
                countIf(started_at > now() - INTERVAL 1 DAY) as views_24h,
                countIf(started_at > now() - INTERVAL 7 DAY) as views_7d,
                views_7d / 7.0 as daily_avg,
                if(daily_avg > 0, views_24h / daily_avg, 0) as velocity,
                avg(completion_rate) as completion_rate_avg
            FROM playback_sessions
            WHERE started_at > now() - INTERVAL 7 DAY
            GROUP BY item_id
            HAVING views_24h >= {}
              AND velocity >= {}
            ORDER BY velocity DESC, views_24h DESC
            LIMIT {}
            "#,
            min_views, min_velocity, limit
        );

        let rows = self.client.inner().query(&query).fetch_all().await?;
        Ok(rows)
    }

    /// Get most popular items in time window
    pub async fn get_popular(
        &self,
        time_window_hours: u32,
        limit: usize,
    ) -> Result<Vec<u32>> {
        let query = format!(
            r#"
            SELECT
                item_id,
                count() as view_count
            FROM playback_sessions
            WHERE started_at > now() - INTERVAL {} HOUR
            GROUP BY item_id
            ORDER BY view_count DESC
            LIMIT {}
            "#,
            time_window_hours, limit
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct PopularItem {
            item_id: u32,
            #[allow(dead_code)]
            view_count: u64,
        }

        let rows: Vec<PopularItem> = self.client.inner().query(&query).fetch_all().await?;
        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Get trending by genre (delegates to get_trending with defaults)
    pub async fn get_trending_by_genre(
        &self,
        _genre: &str,
        limit: usize,
    ) -> Result<Vec<TrendingItem>> {
        // Simplified: genre filter requires item metadata join
        // For now, returns general trending with sensible defaults
        self.get_trending(100, 1.5, limit).await
    }
}
