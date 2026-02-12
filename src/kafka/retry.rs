use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{warn, error};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig;

impl Default for RetryConfig {
    fn default() -> Self {
        Self
    }
}

/// Message that failed processing and needs to go to DLQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterMessage {
    /// Original topic
    pub original_topic: String,
    /// Original partition
    pub original_partition: i32,
    /// Original offset
    pub original_offset: i64,
    /// Original message key
    pub original_key: Option<String>,
    /// Original message payload
    pub payload: Vec<u8>,
    /// Error message
    pub error: String,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Timestamp of first failure
    pub first_failure: chrono::DateTime<chrono::Utc>,
    /// Timestamp of last failure
    pub last_failure: chrono::DateTime<chrono::Utc>,
    /// Consumer group that failed
    pub consumer_group: String,
}

/// Dead Letter Queue handler
pub struct DeadLetterQueue {
    producer: FutureProducer,
    dlq_topic: String,
}

impl DeadLetterQueue {
    pub fn new(
        kafka_brokers: &str,
        dlq_topic: &str,
        _retry_topic: Option<&str>,
        _config: RetryConfig,
    ) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("message.timeout.ms", "5000")
            .set("acks", "all")
            .create()?;

        Ok(Self {
            producer,
            dlq_topic: dlq_topic.to_string(),
        })
    }

    /// Send a message to the dead letter queue
    pub async fn send_to_dlq(&self, message: &DeadLetterMessage) -> Result<()> {
        let payload = serde_json::to_vec(message)?;
        let key = message.original_key.clone().unwrap_or_default();

        let record = FutureRecord::to(&self.dlq_topic)
            .payload(&payload)
            .key(&key);

        match self.producer.send(record, Timeout::After(Duration::from_secs(5))).await {
            Ok(_) => {
                warn!(
                    dlq_topic = %self.dlq_topic,
                    original_topic = %message.original_topic,
                    error = %message.error,
                    retry_count = message.retry_count,
                    "Message sent to dead letter queue"
                );
                Ok(())
            }
            Err((e, _)) => {
                error!(
                    error = ?e,
                    original_topic = %message.original_topic,
                    "CRITICAL: Failed to send message to DLQ"
                );
                Err(anyhow::anyhow!("Failed to send to DLQ: {}", e))
            }
        }
    }
}
