//! Phase 5: Notification Dispatcher Hub
//!
//! Pluggable dispatcher for environment-agnostic notification delivery.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::notification::models::NotificationIntent;
use crate::notification::repository::service::NotificationRepository;

/// Trait for notification delivery adaptors.
#[async_trait]
pub trait NotificationAdaptor: Send + Sync {
    /// Dispatch the notification intent.
    async fn dispatch(&self, intent: &NotificationIntent) -> Result<()>;
    
    /// Get adaptor name.
    fn name(&self) -> &str;
}

/// Adaptor for Kafka-driven real-time dispatching.
pub struct KafkaNotifyAdaptor {
    // In a real implementation, this would hold a Kafka producer.
}

impl Default for KafkaNotifyAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl KafkaNotifyAdaptor {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NotificationAdaptor for KafkaNotifyAdaptor {
    async fn dispatch(&self, intent: &NotificationIntent) -> Result<()> {
        debug!(kind = %intent.kind(), profile_id = %intent.profile_id(), "Dispatching notification via Kafka");
        // Mock Kafka production logic
        Ok(())
    }

    fn name(&self) -> &str { "kafka" }
}

/// Adaptor for poll-based delivery (Local persistence only).
pub struct PollingAdaptor;

impl Default for PollingAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl PollingAdaptor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NotificationAdaptor for PollingAdaptor {
    async fn dispatch(&self, _intent: &NotificationIntent) -> Result<()> {
        // Polling adaptor doesn't do active dispatch; 
        // it relies on the repository having already persisted the data.
        Ok(())
    }

    fn name(&self) -> &str { "polling" }
}

/// Central Hub for notification management.
pub struct NotificationDispatcher {
    repository: Arc<NotificationRepository>,
    adaptor: Arc<dyn NotificationAdaptor>,
}

impl NotificationDispatcher {
    pub fn new(
        repository: Arc<NotificationRepository>,
        adaptor: Arc<dyn NotificationAdaptor>,
    ) -> Self {
        Self { repository, adaptor }
    }

    /// Access the underlying repository for direct querying (e.g. API polling endpoints).
    pub fn repository(&self) -> &Arc<NotificationRepository> {
        &self.repository
    }

    /// Primary entry point: Persist and Dispatch a notification.
    pub async fn dispatch(&self, intent: NotificationIntent) -> Result<()> {
        // 1. Mandatory Persistence (S.O.R + Analytics)
        self.repository.persist(&intent).await?;

        // 2. Pluggable Dispatch
        if let Err(e) = self.adaptor.dispatch(&intent).await {
            warn!(error = %e, adaptor = %self.adaptor.name(), "Notification dispatch failed");
            // We don't fail the whole call because persistence succeeded (SOR is safe)
        }

        info!(kind = %intent.kind(), profile_id = %intent.profile_id(), "Notification processed successfully");
        Ok(())
    }
}
