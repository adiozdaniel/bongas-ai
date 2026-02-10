use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};

use crate::engine::BongasEngine;

pub struct CacheWarmer {
    engine: Arc<BongasEngine>,
    warm_scenarios: Vec<String>,
    warm_interval_minutes: u64,
}

impl CacheWarmer {
    pub fn new(
        engine: Arc<BongasEngine>,
        warm_scenarios: Vec<String>,
        warm_interval_minutes: u64,
    ) -> Self {
        Self {
            engine,
            warm_scenarios,
            warm_interval_minutes,
        }
    }

    /// Start cache warming background task
    pub fn start(self: Arc<Self>) {
        let warmer = self.clone();

        info!(
            warm_interval_minutes = warmer.warm_interval_minutes,
            scenarios = ?warmer.warm_scenarios,
            "Cache warmer started"
        );

        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(warmer.warm_interval_minutes * 60));

            loop {
                timer.tick().await;

                if let Err(e) = warmer.warm_popular_users().await {
                    error!(error = ?e, "Cache warming failed");
                }
            }
        });
    }

    /// Warm caches for popular/active users
    async fn warm_popular_users(&self) -> Result<()> {
        info!("Starting cache warming for popular users");

        let active_users = self.get_active_users(1000).await?;
        let mut warmed = 0;
        let mut failed = 0;

        for user_id in active_users {
            for scenario_slug in &self.warm_scenarios {
                match self.engine.execute_scenario(
                    scenario_slug,
                    Some(user_id),
                    serde_json::Value::Object(serde_json::Map::new()),
                ).await {
                    Ok(_) => {
                        warmed += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        error!(
                            user_id = user_id,
                            scenario_slug = %scenario_slug,
                            error = ?e,
                            "Failed to warm cache"
                        );
                    }
                }
            }
        }

        info!(
            warmed = warmed,
            failed = failed,
            "Cache warming completed"
        );

        Ok(())
    }

    /// Get active users to warm caches for
    async fn get_active_users(&self, _limit: usize) -> Result<Vec<i32>> {
        // In production: query ClickHouse or PostgreSQL for users with recent activity
        // For now, returns empty - will be populated when analytics are connected
        Ok(vec![])
    }
}
