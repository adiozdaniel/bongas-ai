use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;
use tracing::{info, error, warn};

use crate::kafka::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::kafka::retry::{DeadLetterQueue, DeadLetterMessage, RetryConfig};
use crate::kafka::metrics::{ConsumerMetrics};
use crate::engine::staleness_engine::{StalenessEngine, UserEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub user_id: i32,
    pub notification_type: String,  // "new_content", "recommendation", "watchlist_update", "system"
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub priority: Option<String>,  // "high", "normal", "low"
    pub channels: Option<Vec<String>>,  // "push", "email", "in_app"
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub priority: String,
}

/// Trait for notification delivery providers
#[async_trait::async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn send_push(&self, payload: &NotificationPayload) -> Result<()>;
    async fn send_email(&self, payload: &NotificationPayload) -> Result<()>;
    async fn send_in_app(&self, payload: &NotificationPayload) -> Result<()>;
}

/// Default notification provider (stub for actual implementation)
pub struct DefaultNotificationProvider;

#[async_trait::async_trait]
impl NotificationProvider for DefaultNotificationProvider {
    async fn send_push(&self, payload: &NotificationPayload) -> Result<()> {
        // TODO: Integrate with FCM/APNS
        info!(
            user_id = payload.user_id,
            title = %payload.title,
            "Would send push notification"
        );
        Ok(())
    }

    async fn send_email(&self, payload: &NotificationPayload) -> Result<()> {
        // TODO: Integrate with email service (SendGrid, SES, etc.)
        info!(
            user_id = payload.user_id,
            title = %payload.title,
            "Would send email notification"
        );
        Ok(())
    }

    async fn send_in_app(&self, payload: &NotificationPayload) -> Result<()> {
        // TODO: Store in database for in-app notification feed
        info!(
            user_id = payload.user_id,
            title = %payload.title,
            "Would create in-app notification"
        );
        Ok(())
    }
}

pub struct NotificationConsumer {
    consumer: StreamConsumer,
    provider: Arc<dyn NotificationProvider>,
    circuit_breaker: Arc<CircuitBreaker>,
    dlq: Arc<DeadLetterQueue>,
    metrics: Arc<ConsumerMetrics>,
    staleness_engine: Arc<StalenessEngine>,
    topic: String,
    group_id: String,
}

impl NotificationConsumer {
    pub fn new(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
        staleness_engine: Arc<StalenessEngine>,
    ) -> Result<Self> {
        Self::with_provider(
            kafka_brokers,
            group_id,
            topic,
            Arc::new(DefaultNotificationProvider),
            staleness_engine,
        )
    }

    pub fn with_provider(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
        provider: Arc<dyn NotificationProvider>,
        staleness_engine: Arc<StalenessEngine>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "30000")
            .set("max.poll.interval.ms", "300000")
            .create()?;

        consumer.subscribe(&[topic])?;

        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),

            name: format!("notification-consumer-{}", topic),
        }));

        let dlq = Arc::new(DeadLetterQueue::new(
            kafka_brokers,
            &format!("{}.dlq", topic),
            Some(&format!("{}.retry", topic)),
            RetryConfig::default(),
        )?);

        let metrics = Arc::new(ConsumerMetrics::new(group_id, topic));

        Ok(Self {
            consumer,
            provider,
            circuit_breaker,
            dlq,
            metrics,
            staleness_engine,
            topic: topic.to_string(),
            group_id: group_id.to_string(),
        })
    }

    pub async fn start(self: Arc<Self>) {
        info!(topic = %self.topic, "Starting notification consumer");

        loop {
            // Check circuit breaker
            if !self.circuit_breaker.allow_request().await {
                warn!(
                    topic = %self.topic,
                    "Circuit breaker open, waiting before retry"
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            match self.consumer.recv().await {
                Ok(message) => {
                    let payload_len = message.payload().map(|p| p.len()).unwrap_or(0);
                    self.metrics.record_received(payload_len).await;

                    if let Some(payload) = message.payload() {
                        let start = Instant::now();

                        match self.process_message(payload).await {
                            Ok(_) => {
                                self.metrics.record_success(start.elapsed());
                                self.circuit_breaker.record_success().await;

                                // Commit offset on success
                                if let Err(e) = self.consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async) {
                                    warn!(error = ?e, "Failed to commit offset");
                                }

                                info!(
                                    topic = %self.topic,
                                    processing_time_ms = start.elapsed().as_millis(),
                                    "Processed notification"
                                );
                            }
                            Err(e) => {
                                self.metrics.record_failure();
                                self.circuit_breaker.record_failure().await;

                                error!(error = ?e, topic = %self.topic, "Failed to process notification");

                                // Send to DLQ
                                let dlq_message = DeadLetterMessage {
                                    original_topic: self.topic.clone(),
                                    original_partition: message.partition(),
                                    original_offset: message.offset(),
                                    original_key: message.key().map(|k| String::from_utf8_lossy(k).to_string()),
                                    payload: payload.to_vec(),
                                    error: e.to_string(),
                                    retry_count: 0,
                                    first_failure: chrono::Utc::now(),
                                    last_failure: chrono::Utc::now(),
                                    consumer_group: self.group_id.clone(),
                                };

                                if let Err(dlq_err) = self.dlq.send_to_dlq(&dlq_message).await {
                                    error!(error = ?dlq_err, "Failed to send to DLQ");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = ?e, topic = %self.topic, "Kafka consumer error");
                    self.circuit_breaker.record_failure().await;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_message(&self, payload: &[u8]) -> Result<()> {
        let event: NotificationEvent = serde_json::from_slice(payload)?;

        info!(
            user_id = event.user_id,
            notification_type = %event.notification_type,
            "Processing notification"
        );

        let notification_payload = NotificationPayload {
            user_id: event.user_id,
            title: event.title.clone(),
            body: event.body.clone(),
            data: event.data.clone(),
            priority: event.priority.clone().unwrap_or_else(|| "normal".to_string()),
        };

        // Determine channels to use
        let channels = event.channels.unwrap_or_else(|| {
            // Default channels based on notification type
            match event.notification_type.as_str() {
                "new_content" => vec!["push".to_string(), "in_app".to_string()],
                "recommendation" => vec!["in_app".to_string()],
                "watchlist_update" => vec!["push".to_string()],
                "system" => vec!["email".to_string(), "in_app".to_string()],
                _ => vec!["in_app".to_string()],
            }
        });

        let channels_count = channels.len();

        // Send to each channel
        let mut errors = Vec::new();

        for channel in channels {
            let result = match channel.as_str() {
                "push" => self.provider.send_push(&notification_payload).await,
                "email" => self.provider.send_email(&notification_payload).await,
                "in_app" => self.provider.send_in_app(&notification_payload).await,
                _ => {
                    warn!(channel = %channel, "Unknown notification channel");
                    continue;
                }
            };

            if let Err(e) = result {
                error!(
                    channel = %channel,
                    error = ?e,
                    "Failed to send notification via channel"
                );
                errors.push(format!("{}: {}", channel, e));
            }
        }

        if !errors.is_empty() {
            // Return error only if all channels failed
            if errors.len() == channels_count {
                return Err(anyhow::anyhow!("All notification channels failed: {:?}", errors));
            }
            // Log partial failures but consider success
            warn!(errors = ?errors, "Some notification channels failed");
        }

        // For certain notification types, invalidate relevant caches
        match event.notification_type.as_str() {
            "new_content" => {
                // New content might affect trending and genre-based recommendations
                self.staleness_engine.process_event(&UserEvent::NewContentInGenre {
                    genre: "all".to_string(), // Could be extracted from event data
                }).await?;
            }
            "recommendation" => {
                // Recommendation notifications might indicate user engagement
                // Could trigger cache refresh for personalized scenarios
                self.staleness_engine.process_event(&UserEvent::ExplicitFeedback {
                    user_id: event.user_id,
                    item_id: 0, // Not applicable for notifications
                    rating: 0.0, // Not applicable for notifications
                }).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
