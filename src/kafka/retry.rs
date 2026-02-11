use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to add jitter to retry delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given retry attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);

        let delay_ms = base_delay.min(self.max_delay.as_millis() as f64);

        let final_delay = if self.jitter {
            // Add up to 25% jitter
            let jitter_factor = 1.0 + (rand::random::<f64>() * 0.5 - 0.25);
            delay_ms * jitter_factor
        } else {
            delay_ms
        };

        Duration::from_millis(final_delay as u64)
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
    retry_topic: Option<String>,
    config: RetryConfig,
}

impl DeadLetterQueue {
    pub fn new(
        kafka_brokers: &str,
        dlq_topic: &str,
        retry_topic: Option<&str>,
        config: RetryConfig,
    ) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("message.timeout.ms", "5000")
            .set("acks", "all")
            .create()?;

        Ok(Self {
            producer,
            dlq_topic: dlq_topic.to_string(),
            retry_topic: retry_topic.map(|s| s.to_string()),
            config,
        })
    }

    /// Send a message to the retry topic for later processing
    pub async fn send_to_retry(
        &self,
        message: &DeadLetterMessage,
    ) -> Result<()> {
        let topic = match &self.retry_topic {
            Some(t) => t,
            None => {
                warn!("No retry topic configured, sending directly to DLQ");
                return self.send_to_dlq(message).await;
            }
        };

        let payload = serde_json::to_vec(message)?;
        let key = message.original_key.clone().unwrap_or_default();

        let record = FutureRecord::to(topic)
            .payload(&payload)
            .key(&key);

        match self.producer.send(record, Timeout::After(Duration::from_secs(5))).await {
            Ok(_) => {
                info!(
                    topic = %topic,
                    original_topic = %message.original_topic,
                    retry_count = message.retry_count,
                    "Message sent to retry topic"
                );
                Ok(())
            }
            Err((e, _)) => {
                error!(error = ?e, "Failed to send to retry topic, sending to DLQ");
                self.send_to_dlq(message).await
            }
        }
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

    /// Determine if a message should be retried or sent to DLQ
    pub async fn handle_failure(
        &self,
        message: &mut DeadLetterMessage,
    ) -> Result<RetryDecision> {
        if message.retry_count < self.config.max_retries {
            message.retry_count += 1;
            message.last_failure = chrono::Utc::now();

            let delay = self.config.delay_for_attempt(message.retry_count);

            Ok(RetryDecision::Retry { delay })
        } else {
            self.send_to_dlq(message).await?;
            Ok(RetryDecision::SendToDlq)
        }
    }
}

#[derive(Debug)]
pub enum RetryDecision {
    /// Retry after the specified delay
    Retry { delay: Duration },
    /// Message has been sent to DLQ
    SendToDlq,
}

/// Retry executor for processing messages with automatic retry
pub struct RetryExecutor<F, T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
{
    operation: F,
    config: RetryConfig,
    _marker: std::marker::PhantomData<(T, E)>,
}

impl<F, T, E> RetryExecutor<F, T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    pub fn new(operation: F, config: RetryConfig) -> Self {
        Self {
            operation,
            config,
            _marker: std::marker::PhantomData,
        }
    }

    pub async fn execute(&self) -> Result<T, E> {
        let mut attempt = 0;

        loop {
            match (self.operation)().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;

                    if attempt > self.config.max_retries {
                        return Err(e);
                    }

                    let delay = self.config.delay_for_attempt(attempt);
                    warn!(
                        attempt = attempt,
                        max_retries = self.config.max_retries,
                        delay_ms = delay.as_millis(),
                        error = %e,
                        "Operation failed, retrying"
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

/// Simple retry helper function
pub async fn with_retry<F, Fut, T, E>(
    operation: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;

                if attempt > config.max_retries {
                    return Err(e);
                }

                let delay = config.delay_for_attempt(attempt);
                warn!(
                    attempt = attempt,
                    max_retries = config.max_retries,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Operation failed, retrying"
                );

                tokio::time::sleep(delay).await;
            }
        }
    }
}
