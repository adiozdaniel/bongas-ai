//! Phase 5: Notification Dispatcher Hub
//!
//! Pluggable dispatcher for environment-agnostic notification delivery.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use tracing::{info, warn, debug};
use handlebars::Handlebars;
use reqwest::Client;

use crate::notification::models::NotificationIntent;
use crate::notification::repository::service::NotificationRepository;
use crate::config::types::notification::ResendConfig;

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

/// Adaptor for Resend-driven email delivery.
pub struct ResendNotifyAdaptor {
    config: ResendConfig,
    http_client: Client,
}

impl ResendNotifyAdaptor {
    pub fn new(config: ResendConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }
}

#[async_trait]
impl NotificationAdaptor for ResendNotifyAdaptor {
    async fn dispatch(&self, intent: &NotificationIntent) -> Result<()> {
        if let NotificationIntent::Email(payload) = intent {
            debug!(profile_id = %payload.profile_id, "Dispatching notification via Resend");
            
            let body = serde_json::json!({
                "from": format!("{} <{}>", self.config.from_name, self.config.from_email),
                "to": [payload.email.clone()],
                "subject": payload.subject.clone(),
                "html": payload.body_html.clone(),
            });

            let response = self.http_client.post("https://api.resend.com/emails")
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await.unwrap_or_default();
                warn!(error = %err_text, "Resend API returned error");
                return Err(anyhow::anyhow!("Resend API error: {}", err_text));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str { "resend" }
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
    handlebars: Arc<Handlebars<'static>>,
}

impl NotificationDispatcher {
    pub fn new(
        repository: Arc<NotificationRepository>,
        adaptor: Arc<dyn NotificationAdaptor>,
    ) -> Self {
        let hb = Handlebars::new();
        // Register default templates or logic here
        // For now we'll just initialize it
        Self { 
            repository, 
            adaptor,
            handlebars: Arc::new(hb),
        }
    }

    /// Register a template for email/inbox rendering.
    pub fn register_template(&self, _name: &str, _template: &str) -> Result<()> {
        // Handlebars doesn't provide a direct way to register on Arc unless we use a Mutex
        // Since we want this to be fast, we'll assume templates are registered at boot.
        // For this implementation, we'll just allow direct rendering in the dispatch call.
        Ok(())
    }

    /// Access the underlying repository for direct querying (e.g. API polling endpoints).
    pub fn repository(&self) -> &Arc<NotificationRepository> {
        &self.repository
    }

    /// Primary entry point: Persist and Dispatch a notification.
    pub async fn dispatch(&self, mut intent: NotificationIntent) -> Result<()> {
        // 1. Dynamic Rendering (The Semantic Why)
        // If the intent has a reasoning or template metadata, we render the body here.
        if let NotificationIntent::Email(ref mut payload) = intent {
            if let Ok(rendered) = self.handlebars.render_template(&payload.body_html, &payload.metadata) {
                payload.body_html = rendered;
            }
        }

        // 2. Mandatory Persistence (S.O.R + Analytics)
        self.repository.persist(&intent).await?;

        // 3. Pluggable Dispatch
        if let Err(e) = self.adaptor.dispatch(&intent).await {
            warn!(error = %e, adaptor = %self.adaptor.name(), "Notification dispatch failed");
            // We don't fail the whole call because persistence succeeded (SOR is safe)
        }

        info!(kind = %intent.kind(), profile_id = %intent.profile_id(), "Notification processed successfully");
        Ok(())
    }
}
