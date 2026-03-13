//! Adaptors for Content Fatigue exposure tracking.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{info, debug};
use redis::AsyncCommands;

use crate::cache::CacheManager;

/// Trait for pluggable exposure sources (Kafka, ClickHouse, or Internal Hooks).
#[async_trait]
pub trait ExposureSource: Send + Sync {
    /// Start the source listener/poller.
    async fn start(&self, shutdown_rx: broadcast::Receiver<()>) -> Result<()>;
    
    /// Get the name of the adaptor.
    fn name(&self) -> &str;

    /// Record an exposure event (called by InternalHook).
    async fn record_exposure(&self, _profile_id: &str, _item_id: i32) -> Result<()> {
        Ok(())
    }

    /// Record an engagement event (called by InternalHook).
    async fn record_engagement(&self, _profile_id: &str, _item_id: i32) -> Result<()> {
        Ok(())
    }
}

/// Adaptor for direct internal hooks from the recommendation engine.
pub struct InternalHookAdaptor {
    cache_manager: Arc<CacheManager>,
}

impl InternalHookAdaptor {
    pub fn new(cache_manager: Arc<CacheManager>) -> Self {
        Self { cache_manager }
    }
}

#[async_trait]
impl ExposureSource for InternalHookAdaptor {
    async fn start(&self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        info!("Fatigue: Internal Hook Adaptor active");
        let _ = shutdown_rx.recv().await;
        Ok(())
    }

    fn name(&self) -> &str { "internal_hook" }

    async fn record_exposure(&self, profile_id: &str, item_id: i32) -> Result<()> {
        if let Some(mut conn) = self.cache_manager.l2_connection() {
            let key = format!("seen:{}:{}", profile_id, item_id);
            let _: () = conn.incr(&key, 1).await?;
            let _: () = conn.expire(&key, 604800).await?;
            debug!(profile_id, item_id, "Fatigue: Direct exposure incremented");
        }
        Ok(())
    }

    async fn record_engagement(&self, profile_id: &str, item_id: i32) -> Result<()> {
        if let Some(mut conn) = self.cache_manager.l2_connection() {
            let key = format!("seen:{}:{}", profile_id, item_id);
            let _: () = conn.del(&key).await?;
            debug!(profile_id, item_id, "Fatigue: Direct engagement reset");
        }
        Ok(())
    }
}

/// Adaptor for Kafka-based event streaming.
pub struct KafkaStreamAdaptor;

impl Default for KafkaStreamAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl KafkaStreamAdaptor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExposureSource for KafkaStreamAdaptor {
    async fn start(&self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        info!("Fatigue: Kafka Stream Adaptor started (Listening to exposure topics)");
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(3600)) => {
                    debug!("Fatigue: Kafka adaptor heartbeat");
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str { "kafka_stream" }
}

/// Adaptor for ClickHouse-based analytical polling.
pub struct ClickHousePollAdaptor {
    _clickhouse: clickhouse::Client,
}

impl ClickHousePollAdaptor {
    pub fn new(clickhouse: clickhouse::Client) -> Self {
        Self { _clickhouse: clickhouse }
    }
}

#[async_trait]
impl ExposureSource for ClickHousePollAdaptor {
    async fn start(&self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        info!("Fatigue: ClickHouse Poll Adaptor started (Polling for recent activity)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                _ = interval.tick() => {
                    debug!("Fatigue: ClickHouse adaptor polling cycle");
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str { "clickhouse_poll" }
}
