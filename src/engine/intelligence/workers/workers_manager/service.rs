//! Engine pulse workers and background maintenance tasks.

use std::sync::Arc;
use tracing::{info, warn};
use crate::engine::BongasEngine;

impl BongasEngine {
    /// Identify low-performing scenarios based on ClickThrough Rate (CTR) from ClickHouse.
    pub async fn get_low_performing_scenarios(&self) -> Vec<String> {
        if let Some(ref ch) = self.execution.manager.clickhouse {
            info!("Querying ClickHouse for scenario performance...");
            
            let query = r#"
                SELECT 
                    scenario_slug,
                    countIf(interaction_type = 'click') / GREATEST(countIf(interaction_type = 'impression'), 1) as ctr
                FROM user_interactions
                WHERE created_at >= (now() - INTERVAL 7 DAY)
                  AND scenario_slug != 'unknown'
                GROUP BY scenario_slug
                HAVING countIf(interaction_type = 'impression') > 100
                ORDER BY ctr ASC
                LIMIT 3
            "#;

            match ch.query(query).fetch_all::<(String, f64)>().await {
                Ok(results) => {
                    results.into_iter().map(|(slug, _)| slug).collect()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to fetch scenario CTR from ClickHouse user_interactions table");
                    Vec::new()
                }
            }
        } else {
            warn!("ClickHouse not available for performance pruning");
            Vec::new()
        }
    }

    /// Start cache warming background task
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval: std::time::Duration) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_manager = self.execution.manager.cache_manager.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        
        let cache_warmer = Arc::new(crate::cache::warming::CacheWarmer::new(
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
