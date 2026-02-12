use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};

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

        let active_users = match self.get_active_users(1000).await {
            Ok(users) => users,
            Err(e) => {
                warn!(error = ?e, "Failed to get active users, using fallback");
                // Fallback to a small set of test users if ClickHouse query fails
                vec![1, 2, 3, 4, 5]
            }
        };

        let mut warmed = 0;
        let mut failed = 0;
        let mut total_attempts = 0;

        for user_id in active_users {
            for scenario_slug in &self.warm_scenarios {
                total_attempts += 1;
                
                // Retry logic for cache warming
                let mut retry_count = 0;
                let max_retries = 3;
                let mut success = false;

                while retry_count < max_retries && !success {
                    match self.engine.execute_scenario(
                        scenario_slug,
                        Some(user_id),
                        serde_json::Value::Object(serde_json::Map::new()),
                    ).await {
                        Ok(_) => {
                            warmed += 1;
                            success = true;
                        }
                        Err(e) => {
                            retry_count += 1;
                            if retry_count >= max_retries {
                                failed += 1;
                                error!(
                                    user_id = user_id,
                                    scenario_slug = %scenario_slug,
                                    attempt = retry_count,
                                    error = ?e,
                                    "Failed to warm cache after {} attempts",
                                    max_retries
                                );
                            } else {
                                warn!(
                                    user_id = user_id,
                                    scenario_slug = %scenario_slug,
                                    attempt = retry_count,
                                    "Cache warming attempt failed, retrying..."
                                );
                                // Wait before retry
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }
        }

        let success_rate = if total_attempts > 0 {
            (warmed as f64 / total_attempts as f64) * 100.0
        } else {
            0.0
        };

        info!(
            warmed = warmed,
            failed = failed,
            total_attempts = total_attempts,
            success_rate = format!("{:.1}%", success_rate),
            "Cache warming completed"
        );

        // Log warning if success rate is too low
        if success_rate < 50.0 {
            warn!(
                success_rate = format!("{:.1}%", success_rate),
                "Cache warming success rate is low, check system health"
            );
        }

        Ok(())
    }

    /// Get active users to warm caches for
    async fn get_active_users(&self, limit: usize) -> Result<Vec<i32>> {
        // Query ClickHouse for users with recent activity (last 24h)
        let query = format!(
            r#"
            SELECT user_id
            FROM user_interactions
            WHERE created_at >= now() - INTERVAL 1 DAY
            GROUP BY user_id
            ORDER BY max(created_at) DESC
            LIMIT {}
            "#,
            limit
        );

        #[derive(clickhouse::Row, serde::Deserialize)]
        struct ActiveUser {
            user_id: i32,
        }

        let rows: Vec<ActiveUser> = self.engine.clickhouse_client().inner()
            .query(&query)
            .fetch_all()
            .await?;

        let users: Vec<i32> = rows.into_iter().map(|row| row.user_id).collect();

        info!(
            found_users = users.len(),
            requested_limit = limit,
            "Retrieved active users for cache warming"
        );

        Ok(users)
    }
}
