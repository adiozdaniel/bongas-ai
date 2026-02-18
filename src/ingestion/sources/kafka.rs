//! Kafka activity source — real-time stream consumer.
//!
//! Subscribes to Kafka topics and converts native events into `UserActivity`.
//! Uses the global `CircuitBreakerRegistry` (not a duplicate breaker).

use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, error, warn, debug};

use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId, CircuitBreakerConfig};
use super::super::types::{ActivitySource, SourceHealth, UserActivity};
use crate::config::types::kafka as config_kafka;

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

impl From<config_kafka::KafkaConfig> for KafkaSourceConfig {
    fn from(config: config_kafka::KafkaConfig) -> Self {
        KafkaSourceConfig {
            brokers: config.brokers,
            group_id: config.group_id,
            playback_topic: config.playback_topic,
            reaction_topic: config.reaction_topic,
            profile_topic: config.profile_topic,
            notification_topic: config.notification_topic,
        }
    }
}

use rdkafka::producer::{FutureProducer, FutureRecord};

/// Kafka-based activity source.
pub struct KafkaSource {
    config: KafkaSourceConfig,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
    dlq_producer: FutureProducer,
}

impl KafkaSource {
    pub fn new(
        config: KafkaSourceConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        let dlq_producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Kafka DLQ producer creation error");

        Self {
            config,
            circuit_breaker_registry,
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            dlq_producer,
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
            .set("enable.auto.commit", "false") // Manual commit for reliability
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

        info!(topic, "Kafka consumer started with DLQ protection");

        loop {
            // Check if circuit is open before processing
            if breaker.health().state == crate::circuit_breaker::CircuitState::Open {
                warn!(source = "kafka", "Circuit breaker open, pausing");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Use recv() for async message consumption
            match tokio::time::timeout(Duration::from_millis(100), consumer.recv()).await {
                Ok(Ok(message)) => {
                    if let Some(payload) = message.payload() {
                        let mut success = false;
                        let mut attempts = 0;
                        const MAX_ATTEMPTS: u32 = 3;

                        while !success && attempts < MAX_ATTEMPTS {
                            attempts += 1;
                            match parse(payload) {
                                Some(activity) => {
                                    if sender.send(activity).await.is_ok() {
                                        self.messages_ingested.fetch_add(1, Ordering::Relaxed);
                                        success = true;
                                    }
                                }
                                None => {
                                    // Parsing failed - retry or DLQ
                                    if attempts < MAX_ATTEMPTS {
                                        debug!(topic, attempts, "Parsing failed, retrying...");
                                        tokio::time::sleep(Duration::from_millis(50)).await;
                                    }
                                }
                            }
                        }

use rdkafka::consumer::CommitMode;

// ... (in consume_topic loop)
                        if !success {
                            // POISON MESSAGE DETECTED after retries
                            self.errors.fetch_add(1, Ordering::Relaxed);
                            let dlq_topic = format!("{}.dlq", topic);
                            warn!(topic, dlq_topic, attempts, "Poison message failed after retries, moving to DLQ");
                            
                            let record = FutureRecord::to(&dlq_topic)
                                .payload(payload)
                                .key("poison");
                            
                            let _ = self.dlq_producer.send(record, Duration::from_secs(0)).await;
                        }

                        // Manual commit after processing (success or DLQ)
                        let _ = consumer.commit_message(&message, CommitMode::Async);
                    }
                }
                Ok(Err(e)) => {
                    error!(topic, error = %e, "Kafka receive error");
                    self.errors.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(_) => {
                    // Timeout - continue loop
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
                        scenario_slug: None,
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
                        scenario_slug: None,
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
