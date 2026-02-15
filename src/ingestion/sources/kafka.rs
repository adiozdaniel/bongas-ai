//! Kafka activity source — real-time stream consumer.
//!
//! Subscribes to Kafka topics and converts native events into `UserActivity`.
//! Uses the global `CircuitBreakerRegistry` (not a duplicate breaker).

use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, error, warn};

use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId, CircuitBreakerConfig};
use super::super::types::{ActivitySource, SourceHealth, UserActivity};

/// Configuration for the Kafka activity source.
#[derive(Debug, Clone)]
pub struct KafkaSourceConfig {
    pub brokers: String,
    pub group_id: String,
    pub playback_topic: String,
    pub reaction_topic: String,
    pub profile_topic: String,
    pub notification_topic: String,
}

/// Kafka-based activity source.
pub struct KafkaSource {
    config: KafkaSourceConfig,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
}

impl KafkaSource {
    pub fn new(
        config: KafkaSourceConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        Self {
            config,
            circuit_breaker_registry,
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    fn breaker_id() -> CircuitBreakerId {
        CircuitBreakerId::new("ingestion:kafka")
    }

    /// Create a consumer for a specific topic.
    fn create_consumer(&self, topic: &str) -> Result<StreamConsumer> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "30000")
            .create()?;

        consumer.subscribe(&[topic])?;
        Ok(consumer)
    }

    /// Run a consumer loop for a single topic, parsing messages into UserActivity.
    async fn consume_topic<F>(
        &self,
        topic: &str,
        sender: &mpsc::Sender<UserActivity>,
        parse: F,
    ) where
        F: Fn(&[u8]) -> Option<UserActivity>,
    {
        let consumer = match self.create_consumer(topic) {
            Ok(c) => c,
            Err(e) => {
                error!(topic, error = %e, "Failed to create Kafka consumer");
                self.errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        let breaker = self.circuit_breaker_registry.get_or_create(
            Self::breaker_id(),
            CircuitBreakerConfig::default(),
        );

        info!(topic, "Kafka consumer started");

        loop {
            // Check circuit breaker via the global registry
            if breaker.health().state == crate::circuit_breaker::CircuitState::Open {
                warn!(topic, "Circuit breaker open for Kafka source, backing off");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            match consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        if let Some(activity) = parse(payload) {
                            if sender.send(activity).await.is_err() {
                                warn!(topic, "Activity channel closed, stopping consumer");
                                return;
                            }
                            self.messages_ingested.fetch_add(1, Ordering::Relaxed);
                            breaker.record_success();
                        } else {
                            warn!(topic, "Failed to parse Kafka message");
                            self.errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(e) => {
                    error!(topic, error = %e, "Kafka consumer error");
                    self.errors.fetch_add(1, Ordering::Relaxed);
                    breaker.record_failure();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl ActivitySource for KafkaSource {
    fn name(&self) -> &str {
        "kafka"
    }

    async fn start(&self, sender: mpsc::Sender<UserActivity>) -> Result<()> {
        info!(brokers = %self.config.brokers, "Starting Kafka activity source");

        // Spawn a consumer task for each topic
        let playback_sender = sender.clone();
        let playback_topic = self.config.playback_topic.clone();
        let reaction_sender = sender.clone();
        let reaction_topic = self.config.reaction_topic.clone();
        let profile_sender = sender.clone();
        let profile_topic = self.config.profile_topic.clone();
        let notification_sender = sender;
        let notification_topic = self.config.notification_topic.clone();

        // We run all 4 topic consumers concurrently
        tokio::select! {
            _ = self.consume_topic(&playback_topic, &playback_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    Some(UserActivity::Playback {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        item_id: v.get("item_id")?.as_i64()? as i32,
                        session_id: v.get("session_id")?.as_str()?.to_string(),
                        watch_duration_seconds: v.get("watch_duration_seconds")?.as_i64()? as i32,
                        total_duration_seconds: v.get("total_duration_seconds")?.as_i64()? as i32,
                        watch_percentage: v.get("watch_percentage")?.as_f64()? as f32,
                        completed: v.get("completed")?.as_bool()?,
                        timestamp: chrono::Utc::now(),
                    })
                })
            }) => {}
            _ = self.consume_topic(&reaction_topic, &reaction_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    Some(UserActivity::Reaction {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        item_id: v.get("item_id")?.as_i64()? as i32,
                        reaction_type: v.get("reaction_type")?.as_str()?.to_string(),
                        timestamp: chrono::Utc::now(),
                    })
                })
            }) => {}
            _ = self.consume_topic(&profile_topic, &profile_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    Some(UserActivity::ProfileUpdate {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        update_type: v.get("update_type")?.as_str()?.to_string(),
                        data: v.get("data")?.clone(),
                        timestamp: chrono::Utc::now(),
                    })
                })
            }) => {}
            _ = self.consume_topic(&notification_topic, &notification_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    Some(UserActivity::Notification {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        notification_type: v.get("notification_type")?.as_str()?.to_string(),
                        title: v.get("title")?.as_str()?.to_string(),
                        body: v.get("body")?.as_str()?.to_string(),
                        data: v.get("data").cloned().unwrap_or(serde_json::json!({})),
                        priority: v.get("priority").and_then(|p| p.as_str()).unwrap_or("normal").to_string(),
                        channels: v.get("channels")
                            .and_then(|c| serde_json::from_value::<Vec<String>>(c.clone()).ok())
                            .unwrap_or_default(),
                        timestamp: chrono::Utc::now(),
                    })
                })
            }) => {}
        }

        Ok(())
    }

    async fn health(&self) -> SourceHealth {
        let breaker = self.circuit_breaker_registry.get_or_create(
            Self::breaker_id(),
            CircuitBreakerConfig::default(),
        );
        let health = breaker.health();

        SourceHealth {
            source_name: "kafka".to_string(),
            healthy: health.state != crate::circuit_breaker::CircuitState::Open,
            messages_ingested: self.messages_ingested.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            circuit_state: format!("{:?}", health.state),
            last_activity: None,
        }
    }
}
