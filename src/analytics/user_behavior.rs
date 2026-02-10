use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::analytics::clickhouse::ClickHouseClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorStats {
    pub total_watch_time_minutes: u32,
    pub total_videos_watched: u32,
    pub avg_completion_rate: f32,
    pub completion_histogram: HashMap<String, u32>,
    pub active_days_last_30d: u32,
    pub favorite_time_of_day: Option<String>,
}

pub struct UserBehaviorQuery {
    client: Arc<ClickHouseClient>,
}

impl UserBehaviorQuery {
    pub fn new(client: Arc<ClickHouseClient>) -> Self {
        Self { client }
    }

    /// Get comprehensive user behavior statistics
    pub async fn get_user_stats(&self, user_id: u32) -> Result<UserBehaviorStats> {
        // Query 1: Basic stats
        #[derive(clickhouse::Row, Deserialize)]
        struct BasicStats {
            total_watch_time_minutes: u32,
            total_videos_watched: u32,
            avg_completion_rate: f32,
        }

        let basic_rows: Vec<BasicStats> = self.client.inner().query(&format!(
            r#"
            SELECT
                sum(watch_duration_seconds) / 60 as total_watch_time_minutes,
                count(DISTINCT item_id) as total_videos_watched,
                avg(completion_rate) as avg_completion_rate
            FROM playback_sessions
            WHERE user_id = {}
              AND started_at > now() - INTERVAL 30 DAY
            "#,
            user_id
        )).fetch_all().await?;

        let basic = basic_rows.into_iter().next().unwrap_or(BasicStats {
            total_watch_time_minutes: 0,
            total_videos_watched: 0,
            avg_completion_rate: 0.0,
        });

        // Query 2: Completion histogram
        #[derive(clickhouse::Row, Deserialize)]
        struct HistogramRow {
            bucket: String,
            count: u32,
        }

        let histogram_rows: Vec<HistogramRow> = self.client.inner().query(&format!(
            r#"
            SELECT
                multiIf(
                    completion_rate < 0.25, '0-25%',
                    completion_rate < 0.50, '25-50%',
                    completion_rate < 0.75, '50-75%',
                    '75-100%'
                ) as bucket,
                count() as count
            FROM playback_sessions
            WHERE user_id = {}
              AND started_at > now() - INTERVAL 30 DAY
            GROUP BY bucket
            "#,
            user_id
        )).fetch_all().await?;

        let completion_histogram: HashMap<String, u32> = histogram_rows
            .into_iter()
            .map(|row| (row.bucket, row.count))
            .collect();

        // Query 3: Active days
        #[derive(clickhouse::Row, Deserialize)]
        struct ActiveDaysRow {
            active_days: u32,
        }

        let active_days_rows: Vec<ActiveDaysRow> = self.client.inner().query(&format!(
            r#"
            SELECT count(DISTINCT toDate(started_at)) as active_days
            FROM playback_sessions
            WHERE user_id = {}
              AND started_at > now() - INTERVAL 30 DAY
            "#,
            user_id
        )).fetch_all().await?;

        let active_days = active_days_rows
            .into_iter()
            .next()
            .map(|r| r.active_days)
            .unwrap_or(0);

        // Query 4: Favorite time of day
        #[derive(clickhouse::Row, Deserialize)]
        struct TimeOfDayRow {
            time_of_day: String,
            #[allow(dead_code)]
            count: u32,
        }

        let time_rows: Vec<TimeOfDayRow> = self.client.inner().query(&format!(
            r#"
            SELECT
                multiIf(
                    toHour(started_at) >= 6 AND toHour(started_at) < 12, 'morning',
                    toHour(started_at) >= 12 AND toHour(started_at) < 18, 'afternoon',
                    toHour(started_at) >= 18 AND toHour(started_at) < 23, 'evening',
                    'night'
                ) as time_of_day,
                count() as count
            FROM playback_sessions
            WHERE user_id = {}
              AND started_at > now() - INTERVAL 30 DAY
            GROUP BY time_of_day
            ORDER BY count DESC
            LIMIT 1
            "#,
            user_id
        )).fetch_all().await?;

        let favorite_time_of_day = time_rows.into_iter().next().map(|r| r.time_of_day);

        Ok(UserBehaviorStats {
            total_watch_time_minutes: basic.total_watch_time_minutes,
            total_videos_watched: basic.total_videos_watched,
            avg_completion_rate: basic.avg_completion_rate,
            completion_histogram,
            active_days_last_30d: active_days,
            favorite_time_of_day,
        })
    }
}
