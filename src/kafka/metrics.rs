use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use serde::Serialize;

/// Metrics for a single Kafka consumer
#[derive(Debug)]
pub struct ConsumerMetrics {
    /// Consumer name/identifier
    pub name: String,
    /// Topic being consumed
    pub topic: String,
    /// Total messages received
    messages_received: AtomicU64,
    /// Messages processed successfully
    messages_processed: AtomicU64,
    /// Messages that failed processing
    messages_failed: AtomicU64,
    /// Messages sent to retry
    messages_retried: AtomicU64,
    /// Messages sent to DLQ
    messages_dlq: AtomicU64,
    /// Total processing time in microseconds
    processing_time_us: AtomicU64,
    /// Last message timestamp
    last_message_time: RwLock<Option<Instant>>,
    /// Lag (if available)
    current_lag: AtomicU64,
    /// Bytes received
    bytes_received: AtomicU64,
}

impl ConsumerMetrics {
    pub fn new(name: &str, topic: &str) -> Self {
        Self {
            name: name.to_string(),
            topic: topic.to_string(),
            messages_received: AtomicU64::new(0),
            messages_processed: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            messages_retried: AtomicU64::new(0),
            messages_dlq: AtomicU64::new(0),
            processing_time_us: AtomicU64::new(0),
            last_message_time: RwLock::new(None),
            current_lag: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
        }
    }

    /// Record a message received
    pub async fn record_received(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
        *self.last_message_time.write().await = Some(Instant::now());
    }

    /// Record successful processing
    pub fn record_success(&self, processing_time: Duration) {
        self.messages_processed.fetch_add(1, Ordering::Relaxed);
        self.processing_time_us.fetch_add(processing_time.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record failed processing
    pub fn record_failure(&self) {
        self.messages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record retry
    pub fn record_retry(&self) {
        self.messages_retried.fetch_add(1, Ordering::Relaxed);
    }

    /// Record DLQ
    pub fn record_dlq(&self) {
        self.messages_dlq.fetch_add(1, Ordering::Relaxed);
    }

    /// Update lag
    pub fn update_lag(&self, lag: u64) {
        self.current_lag.store(lag, Ordering::Relaxed);
    }

    /// Get snapshot of metrics
    pub async fn snapshot(&self) -> ConsumerMetricsSnapshot {
        let received = self.messages_received.load(Ordering::Relaxed);
        let processed = self.messages_processed.load(Ordering::Relaxed);
        let processing_time = self.processing_time_us.load(Ordering::Relaxed);

        let avg_processing_time_us = if processed > 0 {
            processing_time / processed
        } else {
            0
        };

        let last_message_age = self.last_message_time.read().await
            .map(|t| t.elapsed())
            .unwrap_or(Duration::MAX);

        ConsumerMetricsSnapshot {
            name: self.name.clone(),
            topic: self.topic.clone(),
            messages_received: received,
            messages_processed: processed,
            messages_failed: self.messages_failed.load(Ordering::Relaxed),
            messages_retried: self.messages_retried.load(Ordering::Relaxed),
            messages_dlq: self.messages_dlq.load(Ordering::Relaxed),
            avg_processing_time_us,
            total_processing_time_us: processing_time,
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            current_lag: self.current_lag.load(Ordering::Relaxed),
            last_message_age_secs: last_message_age.as_secs(),
            success_rate: if received > 0 {
                processed as f64 / received as f64
            } else {
                1.0
            },
        }
    }

    /// Reset metrics
    pub async fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_processed.store(0, Ordering::Relaxed);
        self.messages_failed.store(0, Ordering::Relaxed);
        self.messages_retried.store(0, Ordering::Relaxed);
        self.messages_dlq.store(0, Ordering::Relaxed);
        self.processing_time_us.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        *self.last_message_time.write().await = None;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsumerMetricsSnapshot {
    pub name: String,
    pub topic: String,
    pub messages_received: u64,
    pub messages_processed: u64,
    pub messages_failed: u64,
    pub messages_retried: u64,
    pub messages_dlq: u64,
    pub avg_processing_time_us: u64,
    pub total_processing_time_us: u64,
    pub bytes_received: u64,
    pub current_lag: u64,
    pub last_message_age_secs: u64,
    pub success_rate: f64,
}

/// Aggregated metrics for all Kafka consumers
pub struct KafkaMetricsRegistry {
    consumers: RwLock<HashMap<String, Arc<ConsumerMetrics>>>,
    /// Global metrics
    total_messages_received: AtomicU64,
    total_messages_processed: AtomicU64,
    total_messages_failed: AtomicU64,
    start_time: Instant,
}

impl KafkaMetricsRegistry {
    pub fn new() -> Self {
        Self {
            consumers: RwLock::new(HashMap::new()),
            total_messages_received: AtomicU64::new(0),
            total_messages_processed: AtomicU64::new(0),
            total_messages_failed: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Register a new consumer
    pub async fn register_consumer(&self, name: &str, topic: &str) -> Arc<ConsumerMetrics> {
        let metrics = Arc::new(ConsumerMetrics::new(name, topic));
        self.consumers.write().await.insert(name.to_string(), metrics.clone());
        metrics
    }

    /// Get metrics for a specific consumer
    pub async fn get_consumer(&self, name: &str) -> Option<Arc<ConsumerMetrics>> {
        self.consumers.read().await.get(name).cloned()
    }

    /// Record global message received
    pub fn record_global_received(&self) {
        self.total_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record global message processed
    pub fn record_global_processed(&self) {
        self.total_messages_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record global message failed
    pub fn record_global_failed(&self) {
        self.total_messages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get all metrics as JSON
    pub async fn get_all_metrics(&self) -> serde_json::Value {
        let mut consumer_metrics = Vec::new();

        for (_, metrics) in self.consumers.read().await.iter() {
            consumer_metrics.push(metrics.snapshot().await);
        }

        let uptime = self.start_time.elapsed();
        let received = self.total_messages_received.load(Ordering::Relaxed);
        let processed = self.total_messages_processed.load(Ordering::Relaxed);
        let failed = self.total_messages_failed.load(Ordering::Relaxed);

        serde_json::json!({
            "global": {
                "total_messages_received": received,
                "total_messages_processed": processed,
                "total_messages_failed": failed,
                "success_rate": if received > 0 {
                    processed as f64 / received as f64
                } else {
                    1.0
                },
                "uptime_secs": uptime.as_secs(),
                "messages_per_second": if uptime.as_secs() > 0 {
                    received as f64 / uptime.as_secs() as f64
                } else {
                    0.0
                },
            },
            "consumers": consumer_metrics,
        })
    }

    /// Get metrics summary for health check
    pub async fn health_summary(&self) -> KafkaHealthSummary {
        let consumers = self.consumers.read().await;
        let mut unhealthy_consumers = Vec::new();
        let mut total_lag: u64 = 0;

        for (name, metrics) in consumers.iter() {
            let snapshot = metrics.snapshot().await;
            total_lag += snapshot.current_lag;

            // Consider unhealthy if:
            // - Success rate < 90%
            // - No messages in last 5 minutes (if we expect messages)
            // - Lag > 10000
            if snapshot.success_rate < 0.9
                || snapshot.current_lag > 10000
                || (snapshot.messages_received > 0 && snapshot.last_message_age_secs > 300) {
                unhealthy_consumers.push(name.clone());
            }
        }

        let received = self.total_messages_received.load(Ordering::Relaxed);
        let processed = self.total_messages_processed.load(Ordering::Relaxed);

        KafkaHealthSummary {
            healthy: unhealthy_consumers.is_empty(),
            total_consumers: consumers.len(),
            unhealthy_consumers,
            total_lag,
            global_success_rate: if received > 0 {
                processed as f64 / received as f64
            } else {
                1.0
            },
        }
    }
}

impl Default for KafkaMetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KafkaHealthSummary {
    pub healthy: bool,
    pub total_consumers: usize,
    pub unhealthy_consumers: Vec<String>,
    pub total_lag: u64,
    pub global_success_rate: f64,
}

/// Helper for timing message processing
pub struct ProcessingTimer {
    start: Instant,
    metrics: Arc<ConsumerMetrics>,
}

impl ProcessingTimer {
    pub fn start(metrics: Arc<ConsumerMetrics>) -> Self {
        Self {
            start: Instant::now(),
            metrics,
        }
    }

    pub fn success(self) {
        self.metrics.record_success(self.start.elapsed());
    }

    pub fn failure(self) {
        self.metrics.record_failure();
    }
}
