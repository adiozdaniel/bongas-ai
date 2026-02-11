use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;
use tracing::{info, error, warn};

use crate::kafka::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::kafka::retry::{DeadLetterQueue, DeadLetterMessage, RetryConfig};
use crate::kafka::metrics::{ConsumerMetrics, KafkaMetricsRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdateEvent {
    pub user_id: i32,
    pub update_type: String,  // "preferences", "settings", "demographics", "subscription"
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ProfileConsumer {
    consumer: StreamConsumer,
    db_pool: Arc<PgPool>,
    circuit_breaker: Arc<CircuitBreaker>,
    dlq: Arc<DeadLetterQueue>,
    metrics: Arc<ConsumerMetrics>,
    topic: String,
    group_id: String,
}

impl ProfileConsumer {
    pub fn new(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
        db_pool: Arc<PgPool>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")  // Manual commit for reliability
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "30000")
            .set("max.poll.interval.ms", "300000")
            .create()?;

        consumer.subscribe(&[topic])?;

        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            failure_window: Duration::from_secs(60),
            name: format!("profile-consumer-{}", topic),
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
            db_pool,
            circuit_breaker,
            dlq,
            metrics,
            topic: topic.to_string(),
            group_id: group_id.to_string(),
        })
    }

    pub fn with_metrics_registry(
        mut self,
        registry: Arc<KafkaMetricsRegistry>,
    ) -> Self {
        // Register with global registry
        let metrics = Arc::new(ConsumerMetrics::new(&self.group_id, &self.topic));
        self.metrics = metrics;
        self
    }

    pub fn metrics(&self) -> Arc<ConsumerMetrics> {
        self.metrics.clone()
    }

    pub async fn start(self: Arc<Self>) {
        info!(topic = %self.topic, "Starting profile consumer");

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
                                    "Processed profile update"
                                );
                            }
                            Err(e) => {
                                self.metrics.record_failure();
                                self.circuit_breaker.record_failure().await;

                                error!(error = ?e, topic = %self.topic, "Failed to process profile update");

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
                                } else {
                                    self.metrics.record_dlq();
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
        let event: ProfileUpdateEvent = serde_json::from_slice(payload)?;

        info!(
            user_id = event.user_id,
            update_type = %event.update_type,
            "Processing profile update"
        );

        match event.update_type.as_str() {
            "preferences" => self.update_preferences(&event).await?,
            "settings" => self.update_settings(&event).await?,
            "demographics" => self.update_demographics(&event).await?,
            "subscription" => self.update_subscription(&event).await?,
            _ => {
                warn!(
                    user_id = event.user_id,
                    update_type = %event.update_type,
                    "Unknown profile update type"
                );
            }
        }

        Ok(())
    }

    async fn update_preferences(&self, event: &ProfileUpdateEvent) -> Result<()> {
        // Extract preference data
        let preferred_genres: Option<Vec<String>> = event.data.get("preferred_genres")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let preferred_languages: Option<Vec<String>> = event.data.get("preferred_languages")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let content_maturity: Option<String> = event.data.get("content_maturity")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Update user preferences in database
        sqlx::query(
            r#"
            INSERT INTO user_preferences (user_id, preferred_genres, preferred_languages, content_maturity, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                preferred_genres = COALESCE($2, user_preferences.preferred_genres),
                preferred_languages = COALESCE($3, user_preferences.preferred_languages),
                content_maturity = COALESCE($4, user_preferences.content_maturity),
                updated_at = NOW()
            "#
        )
        .bind(event.user_id)
        .bind(preferred_genres.map(|g| serde_json::json!(g)))
        .bind(preferred_languages.map(|l| serde_json::json!(l)))
        .bind(content_maturity)
        .execute(self.db_pool.as_ref())
        .await?;

        info!(user_id = event.user_id, "Updated user preferences");
        Ok(())
    }

    async fn update_settings(&self, event: &ProfileUpdateEvent) -> Result<()> {
        // Extract settings data
        let notifications_enabled: Option<bool> = event.data.get("notifications_enabled")
            .and_then(|v| v.as_bool());

        let autoplay_enabled: Option<bool> = event.data.get("autoplay_enabled")
            .and_then(|v| v.as_bool());

        let video_quality: Option<String> = event.data.get("video_quality")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Update user settings
        sqlx::query(
            r#"
            INSERT INTO user_settings (user_id, notifications_enabled, autoplay_enabled, video_quality, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                notifications_enabled = COALESCE($2, user_settings.notifications_enabled),
                autoplay_enabled = COALESCE($3, user_settings.autoplay_enabled),
                video_quality = COALESCE($4, user_settings.video_quality),
                updated_at = NOW()
            "#
        )
        .bind(event.user_id)
        .bind(notifications_enabled)
        .bind(autoplay_enabled)
        .bind(video_quality)
        .execute(self.db_pool.as_ref())
        .await?;

        info!(user_id = event.user_id, "Updated user settings");
        Ok(())
    }

    async fn update_demographics(&self, event: &ProfileUpdateEvent) -> Result<()> {
        // Extract demographics data
        let age_group: Option<String> = event.data.get("age_group")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let country: Option<String> = event.data.get("country")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timezone: Option<String> = event.data.get("timezone")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Update user demographics
        sqlx::query(
            r#"
            INSERT INTO user_demographics (user_id, age_group, country, timezone, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                age_group = COALESCE($2, user_demographics.age_group),
                country = COALESCE($3, user_demographics.country),
                timezone = COALESCE($4, user_demographics.timezone),
                updated_at = NOW()
            "#
        )
        .bind(event.user_id)
        .bind(age_group)
        .bind(country)
        .bind(timezone)
        .execute(self.db_pool.as_ref())
        .await?;

        info!(user_id = event.user_id, "Updated user demographics");
        Ok(())
    }

    async fn update_subscription(&self, event: &ProfileUpdateEvent) -> Result<()> {
        // Extract subscription data
        let tier: Option<String> = event.data.get("tier")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let is_active: Option<bool> = event.data.get("is_active")
            .and_then(|v| v.as_bool());

        let expires_at: Option<chrono::DateTime<chrono::Utc>> = event.data.get("expires_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        // Update user subscription
        sqlx::query(
            r#"
            INSERT INTO user_subscriptions (user_id, tier, is_active, expires_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id) DO UPDATE SET
                tier = COALESCE($2, user_subscriptions.tier),
                is_active = COALESCE($3, user_subscriptions.is_active),
                expires_at = COALESCE($4, user_subscriptions.expires_at),
                updated_at = NOW()
            "#
        )
        .bind(event.user_id)
        .bind(tier)
        .bind(is_active)
        .bind(expires_at)
        .execute(self.db_pool.as_ref())
        .await?;

        info!(user_id = event.user_id, "Updated user subscription");
        Ok(())
    }
}
