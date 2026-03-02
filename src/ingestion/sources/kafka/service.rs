//! Kafka activity source — real-time stream consumer.
//!
//! Subscribes to Kafka topics and converts native events into `UserActivity`.
//! Uses the global `CircuitBreakerRegistry` (not a duplicate breaker).

use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer, CommitMode};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, error, warn, debug};

use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId, CircuitBreakerConfig};
use crate::error::{ErrorClassification, ErrorClassifier};
use crate::ingestion::types::{ActivitySource, SourceHealth, UserActivity};
use crate::config::types::kafka as config_kafka;
use rdkafka::producer::{FutureProducer, FutureRecord};

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

/// Kafka-based activity source.
pub struct KafkaSource {
    config: KafkaSourceConfig,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
    dlq_producer: Option<FutureProducer>,
    fail_count: AtomicU64,
}

#[derive(Debug, thiserror::Error)]
enum KafkaSourceError {
    #[error("Kafka receive error: {0}")]
    Receive(#[from] rdkafka::error::KafkaError),
    #[error("Poison message: parsing failed")]
    Poison(Vec<u8>),
    #[error("Channel full: failed to send activity")]
    ChannelFull,
}

impl ErrorClassifier for KafkaSourceError {
    fn classify(&self) -> ErrorClassification {
        match self {
            Self::Receive(_) => ErrorClassification::Transient,
            Self::Poison(_) => ErrorClassification::Permanent,
            Self::ChannelFull => ErrorClassification::Overload,
        }
    }
}

impl KafkaSource {
    pub fn new(
        config: KafkaSourceConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        let dlq_result: Result<FutureProducer, rdkafka::error::KafkaError> = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("message.timeout.ms", "5000")
            .set("security.protocol", "plaintext")
            .set("api.version.request", "true")
            .set("broker.address.family", "v4")
            .create();

        let dlq_producer = match dlq_result {
            Ok(p) => Some(p),
            Err(e) => {
                error!("Kafka DLQ producer creation failed: {:?}. DLQ functionality disabled.", e);
                None
            }
        };

        Self {
            config,
            circuit_breaker_registry,
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            dlq_producer,
            fail_count: AtomicU64::new(0),
        }
    }

    fn breaker_id() -> CircuitBreakerId {
        CircuitBreakerId::new("ingestion:kafka")
    }

    /// Ensure all required topics and their DLQs exist.
    async fn ensure_topics(&self) -> Result<()> {
        let admin_client: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", &self.config.brokers)
            .set("security.protocol", "plaintext")
            .set("api.version.request", "true")
            .set("broker.address.family", "v4")
            .create()?;

        let topics = vec![
            &self.config.playback_topic,
            &self.config.reaction_topic,
            &self.config.profile_topic,
            &self.config.notification_topic,
        ];

        let mut topic_names = Vec::new();
        for topic in topics {
            topic_names.push(topic.clone());
            topic_names.push(format!("{}.dlq", topic));
        }
        topic_names.push("recommendations.sync".to_string());

        let mut new_topics = Vec::new();
        for name in &topic_names {
            new_topics.push(NewTopic::new(name, 1, TopicReplication::Fixed(1)));
        }

        let options = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));
        
        match admin_client.create_topics(&new_topics, &options).await {
            Ok(results) => {
                for result in results {
                    match result {
                        Ok(t) => info!(topic = %t, "Created Kafka topic"),
                        Err((t, rdkafka::types::RDKafkaErrorCode::TopicAlreadyExists)) => {
                            debug!(topic = %t, "Kafka topic already exists");
                        }
                        Err((t, e)) => {
                            warn!(topic = %t, error = ?e, "Failed to create Kafka topic");
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = ?e, "Failed to execute create_topics request. Broker might be unreachable or auto-topic-creation disabled.");
            }
        }

        Ok(())
    }

    /// Calculate backoff duration based on failure count
    fn get_backoff_duration(&self, fail_count: u64) -> Duration {
        match fail_count {
            0 => Duration::from_secs(0),
            1 => Duration::from_secs(300),  // 5 mins
            2 => Duration::from_secs(600),  // 10 mins
            3 => Duration::from_secs(900),  // 15 mins
            4 => Duration::from_secs(10800), // 3 hours
            _ => Duration::from_secs(86400), // Daily
        }
    }

    /// Create a consumer for a specific topic.
    fn create_consumer(&self, topic: &str) -> Result<StreamConsumer> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "30000")
            .set("security.protocol", "plaintext")
            .set("api.version.request", "true")
            .set("broker.address.family", "v4")
            .create()?;

        consumer.subscribe(&[topic])?;
        Ok(consumer)
    }

    /// Run a consumer loop for a single topic, parsing messages into UserActivity.
    async fn consume_topic<F>(
        &self,
        topic: String,
        sender: mpsc::Sender<UserActivity>,
        parse: F,
    ) where
        F: Fn(&[u8]) -> Option<UserActivity> + Send + Sync + 'static,
    {
        loop {
            let consumer = match self.create_consumer(&topic) {
                Ok(c) => {
                    self.fail_count.store(0, Ordering::Relaxed);
                    Arc::new(c)
                }
                Err(e) => {
                    let count = self.fail_count.fetch_add(1, Ordering::Relaxed) + 1;
                    let backoff = self.get_backoff_duration(count);
                    error!(
                        topic = %topic, 
                        error_desc = %e,
                        error_variant = ?e,
                        fail_count = count,
                        next_retry_secs = backoff.as_secs(),
                        "Failed to create Kafka consumer. Entering progressive backoff."
                    );
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            let breaker = self.circuit_breaker_registry.get_or_create(
                Self::breaker_id(),
                CircuitBreakerConfig::default(),
            );

            info!(topic = %topic, "Kafka consumer started with DLQ protection and Circuit Breaker");

            loop {
                let consumer_inner = consumer.clone();
                let sender_inner = sender.clone();
                let parse_inner = &parse;
                let topic_name = topic.clone();
                let this = self;

                let result = breaker.call(|| async move {
                    let message = tokio::time::timeout(Duration::from_millis(500), consumer_inner.recv())
                        .await
                        .map_err(|_| rdkafka::error::KafkaError::NoMessageReceived)??;

                    let payload = message.payload().ok_or_else(|| KafkaSourceError::Poison(Vec::new()))?;
                    
                    let activity = parse_inner(payload).ok_or_else(|| KafkaSourceError::Poison(payload.to_vec()))?;
                    
                    sender_inner.send(activity).await.map_err(|_| KafkaSourceError::ChannelFull)?;
                    
                    let _ = consumer_inner.commit_message(&message, CommitMode::Async);
                    Ok::<(), KafkaSourceError>(())
                }).await;

                match result {
                    Ok(_) => {
                        this.fail_count.store(0, Ordering::Relaxed);
                        this.messages_ingested.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, classification, .. }) => {
                        this.errors.fetch_add(1, Ordering::Relaxed);
                        
                        if let KafkaSourceError::Receive(rdkafka::error::KafkaError::NoMessageReceived) = source {
                            // Normal timeout, just continue
                            continue;
                        }

                        if classification == ErrorClassification::Permanent {
                            warn!(topic = %topic_name, "Poison message detected, moving to DLQ");
                            
                            if let Some(ref producer) = this.dlq_producer {
                                let payload = if let KafkaSourceError::Poison(ref p) = source {
                                    p.as_slice()
                                } else {
                                    b"unparseable"
                                };

                                let dlq_topic = format!("{}.dlq", topic_name);
                                let record = FutureRecord::to(&dlq_topic)
                                    .payload(payload)
                                    .key("poison_key");
                                
                                let _ = producer.send::<str, [u8], _>(record, Duration::from_secs(0));
                            }
                        } else {
                            let is_connection_error = match &source {
                                KafkaSourceError::Receive(ke) => !matches!(ke, rdkafka::error::KafkaError::NoMessageReceived),
                                _ => false,
                            };

                            if is_connection_error {
                                let count = this.fail_count.fetch_add(1, Ordering::Relaxed) + 1;
                                let backoff = this.get_backoff_duration(count);
                                error!(
                                    topic = %topic_name, 
                                    error_desc = %source,
                                    error_variant = ?source,
                                    fail_count = count,
                                    next_retry_secs = backoff.as_secs(),
                                    "Kafka connection lost. Breaking consumer loop for backoff."
                                );
                                tokio::time::sleep(backoff).await;
                                break; // Break inner loop to re-create consumer
                            } else {
                                error!(topic = %topic_name, error = %source, "Kafka processing error");
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                    Err(crate::circuit_breaker::CircuitBreakerError::Rejected { .. }) => {
                        debug!(topic = %topic_name, "Kafka circuit open, pausing consumption");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(e) => {
                        error!(topic = %topic_name, error = ?e, "Unhandled circuit breaker error in Kafka source");
                    }
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
        info!(brokers = %self.config.brokers, "Starting Kafka activity source with multi-topic isolation");

        let _ = self.ensure_topics().await;

        let p_topic = self.config.playback_topic.clone();
        let p_sender = sender.clone();
        
        let r_topic = self.config.reaction_topic.clone();
        let r_sender = sender.clone();
        
        let pr_topic = self.config.profile_topic.clone();
        let pr_sender = sender.clone();
        
        let n_topic = self.config.notification_topic.clone();
        let n_sender = sender;

        tokio::join!(
            self.consume_topic(p_topic, p_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    let ts = v.get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now);

                    Some(UserActivity::Playback {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        item_id: v.get("item_id")?.as_i64()? as i32,
                        session_id: v.get("session_id")?.as_str()?.to_string(),
                        visitor_id: v.get("visitor_id").and_then(|id| id.as_str()).map(|s| s.to_string()),
                        device_hash: v.get("device_hash").and_then(|h| h.as_str()).map(|s| s.to_string()),
                        watch_duration_seconds: v.get("watch_duration_seconds")?.as_i64()? as i32,
                        total_duration_seconds: v.get("total_duration_seconds")?.as_i64()? as i32,
                        watch_percentage: v.get("watch_percentage")?.as_f64()? as f32,
                        completed: v.get("completed")?.as_bool()?,
                        scenario_slug: None,
                        timestamp: ts,
                    })
                })
            }),
            self.consume_topic(r_topic, r_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    let ts = v.get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now);

                    Some(UserActivity::Reaction {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        item_id: v.get("item_id")?.as_i64()? as i32,
                        visitor_id: v.get("visitor_id").and_then(|id| id.as_str()).map(|s| s.to_string()),
                        device_hash: v.get("device_hash").and_then(|h| h.as_str()).map(|s| s.to_string()),
                        reaction_type: v.get("reaction_type")?.as_str()?.to_string(),
                        scenario_slug: None,
                        timestamp: ts,
                    })
                })
            }),
            self.consume_topic(pr_topic, pr_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    let ts = v.get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now);

                    Some(UserActivity::ProfileUpdate {
                        user_id: v.get("user_id")?.as_i64()? as i32,
                        update_type: v.get("update_type")?.as_str()?.to_string(),
                        data: v.get("data")?.clone(),
                        timestamp: ts,
                    })
                })
            }),
            self.consume_topic(n_topic, n_sender, |payload| {
                serde_json::from_slice::<serde_json::Value>(payload).ok().and_then(|v| {
                    let ts = v.get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now);

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
                        timestamp: ts,
                    })
                })
            })
        );

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
