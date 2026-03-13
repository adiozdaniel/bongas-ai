//! Phase 4: Reasoning Worker
//!
//! Proactively generates and caches human-readable reasons for recommendations.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};
use anyhow::Result;
use serde_json::json;

use crate::cache::CacheManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::db::ItemFeatureService;

pub struct ReasoningWorker {
    hive_mind: Arc<HiveMindConnector>,
    cache_manager: Arc<CacheManager>,
    item_feature_service: Arc<ItemFeatureService>,
    clickhouse: Option<clickhouse::Client>,
    interval: Duration,
}

impl ReasoningWorker {
    pub fn new(
        hive_mind: Arc<HiveMindConnector>,
        cache_manager: Arc<CacheManager>,
        item_feature_service: Arc<ItemFeatureService>,
        clickhouse: Option<clickhouse::Client>,
        interval: Duration,
    ) -> Self {
        Self {
            hive_mind,
            cache_manager,
            item_feature_service,
            clickhouse,
            interval,
        }
    }

    /// Start the background reasoning generation loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        if self.clickhouse.is_none() {
            info!("Reasoning Worker disabled (ClickHouse not configured)");
            return;
        }

        info!(
            interval_mins = self.interval.as_secs() / 60,
            "Reasoning Worker started"
        );

        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_reasoning_cycle().await {
                        error!(error = %e, "Reasoning cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Reasoning Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn run_reasoning_cycle(&self) -> Result<()> {
        debug!("Running reasoning generation cycle...");
        let ch = self.clickhouse.as_ref().unwrap();

        // 1. Find high-probability profile/item pairs from ClickHouse (Mocked Query)
        let query = r#"
            SELECT profile_id, item_id 
            FROM user_interactions 
            WHERE interaction_type = 'click' 
            GROUP BY profile_id, item_id 
            HAVING count() > 2 
            LIMIT 50
        "#;

        let pairs: Vec<(String, i32)> = match ch.query(query).fetch_all().await {
            Ok(res) => res,
            Err(_) => Vec::new(),
        };

        for (pid, iid) in pairs {
            // Check if reason already exists in Redis
            let key = format!("reason:{}:{}", pid, iid);
            let existing: Option<String> = self.cache_manager.get(&key, "reasoning_worker", None, Some(&pid)).await?;
            
            if existing.is_none() {
                // 2. Fetch Features
                let profile = self.item_feature_service.get_profile_features(&pid).await?;
                let item = self.item_feature_service.get_item_features_batch(&[iid]).await?;

                if let (Some(p), Some(i)) = (profile, item.get(&iid)) {
                    // 3. Generate Reason via HiveMind
                    let empty_json = json!({});
                    let empty_array = json!([]);
                    let reason = self.hive_mind.generate_reasoning(
                        &pid, 
                        iid, 
                        p.genre_affinity.as_ref().unwrap_or(&empty_json), 
                        i.tags.as_ref().unwrap_or(&empty_array)
                    ).await?;

                    // 4. Cache in Redis (7 days TTL)
                    self.cache_manager.set_with_ttl(
                        &key, 
                        &reason, 
                        Duration::from_secs(604800),
                        "reasoning_worker",
                        None,
                        Some(&pid)
                    ).await?;
                    
                    debug!(profile_id = %pid, item_id = iid, "Cached generated reason");
                }
            }
        }

        Ok(())
    }
}
