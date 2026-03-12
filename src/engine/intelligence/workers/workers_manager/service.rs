//! Engine pulse workers and background maintenance tasks.

use std::sync::Arc;
use tracing::{info, warn};
use crate::engine::coordination::service::BongasEngine;

/// 💓 Workers: Background maintenance and task orchestration.
pub struct WorkersManager;

impl Default for WorkersManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkersManager {
    pub fn new() -> Self {
        Self
    }
}

impl BongasEngine {
    /// Identify low-performing scenarios based on ClickThrough Rate (CTR) from ClickHouse.
    pub async fn get_low_performing_scenarios(&self) -> Vec<String> {
        if let Some(ref ch) = self.execution.manager.clickhouse {
            info!("Querying ClickHouse for scenario performance...");
            
            let query = r#"
                SELECT 
                    scenario_slug,
                    countIf(interaction_type = 'click') / countIf(interaction_type = 'impression') as ctr
                FROM user_interactions
                WHERE created_at > (toUnixTimestamp(now()) - 86400)
                GROUP BY scenario_slug
                HAVING ctr < 0.02
                ORDER BY ctr ASC
            "#;

            let results: Vec<(String, f64)> = match ch.query(query).fetch_all().await {
                Ok(res) => res,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch low-performing scenarios from ClickHouse");
                    return vec![];
                }
            };

            results.into_iter().map(|(slug, _)| slug).collect()
        } else {
            vec![]
        }
    }

    /// Start cache warming background task
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval: std::time::Duration) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_manager = self.execution.manager.cache_manager.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        
        let cache_warmer = Arc::new(crate::cache::CacheWarmer::new(
            cache_manager,
            scenarios_clone,
            interval,
            shutdown_rx,
        ));

        tokio::spawn(async move {
            cache_warmer.start().await;
        });

        info!(
            scenarios = ?warm_scenarios,
            interval_secs = interval.as_secs(),
            "Cache warming started"
        );
    }
}
