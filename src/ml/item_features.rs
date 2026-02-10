use anyhow::Result;
use serde::Deserialize;
use std::sync::Arc;

use crate::analytics::Analytics;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::models::ItemFeatures;

pub struct ItemFeatureComputer {
    analytics: Arc<Analytics>,
    feature_repo: Arc<FeatureRepository>,
}

impl ItemFeatureComputer {
    pub fn new(analytics: Arc<Analytics>, feature_repo: Arc<FeatureRepository>) -> Self {
        Self { analytics, feature_repo }
    }

    /// Compute all features for an item
    pub async fn compute(&self, item_id: i32) -> Result<ItemFeatures> {
        let stats = self.get_item_stats(item_id).await?;
        let trending_score = self.compute_trending_score(&stats);

        let features = ItemFeatures {
            item_id,
            title: None,
            description: None,
            genres: None,
            tags: None,
            duration_seconds: None,
            tfidf_vector: None,
            view_count: stats.view_count as i32,
            like_count: 0,
            completion_rate: stats.avg_completion_rate,
            trending_score,
            published_at: None,
            features_updated_at: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };

        self.feature_repo.upsert_item_features(item_id, &features).await?;

        Ok(features)
    }

    async fn get_item_stats(&self, item_id: i32) -> Result<ItemStats> {
        let query = format!(
            r#"
            SELECT
                countIf(started_at > now() - INTERVAL 7 DAY) as views_7d,
                countIf(started_at > now() - INTERVAL 30 DAY) as views_30d,
                avg(completion_rate) as avg_completion_rate
            FROM playback_sessions
            WHERE item_id = {}
            "#,
            item_id
        );

        #[derive(clickhouse::Row, Deserialize)]
        struct StatsRow {
            views_7d: u64,
            views_30d: u64,
            avg_completion_rate: f32,
        }

        let rows: Vec<StatsRow> = self.analytics.client.inner().query(&query).fetch_all().await?;
        let row = rows.into_iter().next().unwrap_or(StatsRow {
            views_7d: 0,
            views_30d: 0,
            avg_completion_rate: 0.0,
        });

        Ok(ItemStats {
            view_count: row.views_30d,
            views_7d: row.views_7d,
            avg_completion_rate: row.avg_completion_rate,
        })
    }

    fn compute_trending_score(&self, stats: &ItemStats) -> f32 {
        // Trending = (7d views / 30d views) * completion_rate * 100
        let recency_factor = if stats.view_count > 0 {
            stats.views_7d as f32 / stats.view_count as f32
        } else {
            0.0
        };
        recency_factor * stats.avg_completion_rate * 100.0
    }
}

#[derive(Debug)]
struct ItemStats {
    view_count: u64,
    views_7d: u64,
    avg_completion_rate: f32,
}
