//! API activity source — receives activities from direct HTTP endpoints.
//!
//! Unlike Kafka/ClickHouse (which are pull-based), this source is push-based:
//! API handlers call `ApiSource::ingest()` to feed activities into the pipeline.

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::ingestion::types::{ActivitySource, SourceHealth, UserActivity};

/// API-based activity source for direct endpoint ingestion.
///
/// API handlers (e.g. user-reaction endpoint) call `ingest()` to push
/// activities directly into the processing pipeline.
pub struct ApiSource {
    /// Sender half kept for `ingest()` calls from handlers.
    sender: tokio::sync::RwLock<Option<mpsc::Sender<UserActivity>>>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
}

impl ApiSource {
    pub fn new() -> Self {
        Self {
            sender: tokio::sync::RwLock::new(None),
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    /// Push an activity into the pipeline from an API handler.
    ///
    /// Returns `true` if the activity was accepted, `false` if the channel is full or closed.
    pub async fn ingest(&self, activity: UserActivity) -> bool {
        let sender = self.sender.read().await;
        match sender.as_ref() {
            Some(tx) => match tx.try_send(activity) {
                Ok(()) => {
                    self.messages_ingested.fetch_add(1, Ordering::Relaxed);
                    true
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("API ingestion channel full, activity dropped");
                    self.errors.fetch_add(1, Ordering::Relaxed);
                    false
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    warn!("API ingestion channel closed");
                    self.errors.fetch_add(1, Ordering::Relaxed);
                    false
                }
            },
            None => {
                warn!("API source not started yet, activity dropped");
                self.errors.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }
}

#[async_trait::async_trait]
impl ActivitySource for ApiSource {
    fn name(&self) -> &str {
        "api"
    }

    async fn start(&self, sender: mpsc::Sender<UserActivity>) -> anyhow::Result<()> {
        info!("API activity source started");
        // Store the sender so `ingest()` can use it
        *self.sender.write().await = Some(sender);

        // This source is push-based — nothing to poll.
        // We just park here until cancelled.
        tokio::signal::ctrl_c().await.ok();
        Ok(())
    }

    async fn health(&self) -> SourceHealth {
        let sender = self.sender.read().await;
        SourceHealth {
            source_name: "api".to_string(),
            healthy: sender.is_some(),
            messages_ingested: self.messages_ingested.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            circuit_state: "N/A".to_string(),
            last_activity: None,
        }
    }
}
