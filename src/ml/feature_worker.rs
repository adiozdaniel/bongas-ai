use anyhow::Result;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};

use crate::analytics::Analytics;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::ml::user_features::UserFeatureComputer;
use crate::ml::item_features::ItemFeatureComputer;
use crate::cache::redis::RedisClient;

pub struct FeatureWorker {
    analytics: Arc<Analytics>,
    feature_repo: Arc<FeatureRepository>,
    user_computer: Arc<UserFeatureComputer>,
    item_computer: Arc<ItemFeatureComputer>,
    redis: Arc<RedisClient>,
}

impl FeatureWorker {
    pub fn new(
        analytics: Arc<Analytics>,
        db_pool: PgPool,
        redis: Arc<RedisClient>,
    ) -> Self {
        let feature_repo = Arc::new(FeatureRepository::new(db_pool));

        Self {
            analytics: analytics.clone(),
            feature_repo: feature_repo.clone(),
            user_computer: Arc::new(UserFeatureComputer::new(analytics.clone(), feature_repo.clone())),
            item_computer: Arc::new(ItemFeatureComputer::new(analytics, feature_repo.clone())),
            redis,
        }
    }

    /// Start the feature worker (runs hourly)
    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(3600));

            loop {
                timer.tick().await;

                info!("Starting hourly feature aggregation...");

                match self.run_aggregation().await {
                    Ok(stats) => {
                        info!(
                            "Feature aggregation complete: {} users, {} items in {}s",
                            stats.users_updated, stats.items_updated, stats.duration_seconds
                        );
                    }
                    Err(e) => {
                        error!("Feature aggregation failed: {}", e);
                    }
                }
            }
        });
    }

    /// Run full feature aggregation
    pub async fn run_aggregation(&self) -> Result<AggregationStats> {
        let start = std::time::Instant::now();

        info!("Computing user features...");
        let users_updated = self.compute_user_features().await?;

        info!("Computing item features...");
        let items_updated = self.compute_item_features().await?;

        info!("Caching hot features in Redis...");
        self.cache_hot_features().await?;

        let duration = start.elapsed();

        Ok(AggregationStats {
            users_updated,
            items_updated,
            duration_seconds: duration.as_secs(),
        })
    }

    async fn compute_user_features(&self) -> Result<usize> {
        let active_users = self.get_active_users().await?;

        let mut count = 0;
        for user_id in active_users {
            match self.user_computer.compute(user_id).await {
                Ok(_) => count += 1,
                Err(e) => error!("Failed to compute features for user {}: {}", user_id, e),
            }
        }

        Ok(count)
    }

    async fn compute_item_features(&self) -> Result<usize> {
        let active_items = self.get_active_items().await?;

        let mut count = 0;
        for item_id in active_items {
            match self.item_computer.compute(item_id).await {
                Ok(_) => count += 1,
                Err(e) => error!("Failed to compute features for item {}: {}", item_id, e),
            }
        }

        Ok(count)
    }

    async fn cache_hot_features(&self) -> Result<()> {
        // Cache top 10K most active users
        let hot_users = self.get_hot_users(10000).await?;
        for user_id in hot_users {
            if let Ok(Some(features)) = self.feature_repo.get_user_features(user_id).await {
                let key = format!("user:{}:features", user_id);
                let value = serde_json::to_string(&features)?;
                self.redis.set_ex(&key, &value, 3600).await?;
            }
        }

        // Cache top 5K most viewed items
        let hot_items = self.get_hot_items(5000).await?;
        for item_id in hot_items {
            if let Ok(Some(features)) = self.feature_repo.get_item_features(item_id).await {
                let key = format!("item:{}:features", item_id);
                let value = serde_json::to_string(&features)?;
                self.redis.set_ex(&key, &value, 3600).await?;
            }
        }

        Ok(())
    }

    async fn get_active_users(&self) -> Result<Vec<i32>> {
        #[derive(clickhouse::Row, Deserialize)]
        struct UserRow { user_id: u32 }

        let rows: Vec<UserRow> = self.analytics.client.inner()
            .query("SELECT DISTINCT user_id FROM playback_sessions WHERE started_at > now() - INTERVAL 30 DAY")
            .fetch_all()
            .await?;
        Ok(rows.into_iter().map(|r| r.user_id as i32).collect())
    }

    async fn get_active_items(&self) -> Result<Vec<i32>> {
        #[derive(clickhouse::Row, Deserialize)]
        struct ItemRow { item_id: u32 }

        let rows: Vec<ItemRow> = self.analytics.client.inner()
            .query("SELECT DISTINCT item_id FROM playback_sessions WHERE started_at > now() - INTERVAL 30 DAY")
            .fetch_all()
            .await?;
        Ok(rows.into_iter().map(|r| r.item_id as i32).collect())
    }

    async fn get_hot_users(&self, limit: usize) -> Result<Vec<i32>> {
        #[derive(clickhouse::Row, Deserialize)]
        struct UserRow { user_id: u32, #[allow(dead_code)] view_count: u64 }

        let query = format!(
            "SELECT user_id, count() as view_count FROM playback_sessions WHERE started_at > now() - INTERVAL 7 DAY GROUP BY user_id ORDER BY view_count DESC LIMIT {}",
            limit
        );
        let rows: Vec<UserRow> = self.analytics.client.inner().query(&query).fetch_all().await?;
        Ok(rows.into_iter().map(|r| r.user_id as i32).collect())
    }

    async fn get_hot_items(&self, limit: usize) -> Result<Vec<i32>> {
        #[derive(clickhouse::Row, Deserialize)]
        struct ItemRow { item_id: u32, #[allow(dead_code)] view_count: u64 }

        let query = format!(
            "SELECT item_id, count() as view_count FROM playback_sessions WHERE started_at > now() - INTERVAL 7 DAY GROUP BY item_id ORDER BY view_count DESC LIMIT {}",
            limit
        );
        let rows: Vec<ItemRow> = self.analytics.client.inner().query(&query).fetch_all().await?;
        Ok(rows.into_iter().map(|r| r.item_id as i32).collect())
    }
}

#[derive(Debug)]
pub struct AggregationStats {
    pub users_updated: usize,
    pub items_updated: usize,
    pub duration_seconds: u64,
}
