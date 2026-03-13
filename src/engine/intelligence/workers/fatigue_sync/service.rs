//! Phase 3: Content Fatigue Synchronizer
//!
//! Synchronizes exposure state across the environment to support multi-source fatigue filtering.

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use anyhow::Result;

use crate::cache::CacheManager;
use crate::config::types::ml::ExposureSourceAdaptor;
use crate::engine::intelligence::workers::fatigue_sync::adaptors::{
    ExposureSource, InternalHookAdaptor, KafkaStreamAdaptor, ClickHousePollAdaptor
};

/// Orchestrates pluggable exposure sources for fatigue state synchronization.
pub struct FatigueSynchronizer {
    adaptor: Arc<dyn ExposureSource>,
}

impl FatigueSynchronizer {
    pub fn new(
        config: ExposureSourceAdaptor,
        cache_manager: Arc<CacheManager>,
        clickhouse: Option<clickhouse::Client>,
    ) -> Result<Self> {
        let adaptor: Arc<dyn ExposureSource> = match config {
            ExposureSourceAdaptor::InternalHook => {
                Arc::new(InternalHookAdaptor::new(cache_manager))
            }
            ExposureSourceAdaptor::KafkaStream => {
                Arc::new(KafkaStreamAdaptor::new())
            }
            ExposureSourceAdaptor::ClickHousePoll => {
                let ch = clickhouse.ok_or_else(|| anyhow::anyhow!("ClickHouse client required for ClickHousePoll adaptor"))?;
                Arc::new(ClickHousePollAdaptor::new(ch))
            }
        };

        Ok(Self { adaptor })
    }

    /// Record exposure for a list of items (called by InternalHook).
    pub async fn record_exposures(&self, profile_id: &str, item_ids: Vec<i32>) {
        for item_id in item_ids {
            if let Err(e) = self.adaptor.record_exposure(profile_id, item_id).await {
                warn!(error = %e, profile_id, item_id, "Failed to record exposure");
            }
        }
    }

    /// Record engagement for an item (called by InternalHook).
    pub async fn record_engagement(&self, profile_id: &str, item_id: i32) {
        if let Err(e) = self.adaptor.record_engagement(profile_id, item_id).await {
            warn!(error = %e, profile_id, item_id, "Failed to record engagement");
        }
    }

    /// Start the selected fatigue adaptor.
    pub async fn start(self: Arc<Self>, shutdown_rx: broadcast::Receiver<()>) {
        info!(adaptor = %self.adaptor.name(), "Fatigue Synchronizer starting...");
        if let Err(e) = self.adaptor.start(shutdown_rx).await {
            warn!(error = %e, adaptor = %self.adaptor.name(), "Fatigue adaptor failed");
        }
    }
}
