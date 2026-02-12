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
    pub _name: String,
    /// Topic being consumed
    pub _topic: String,
    /// Total messages received
    messages_received: AtomicU64,
    /// Messages processed successfully
    messages_processed: AtomicU64,
    /// Messages that failed processing
    messages_failed: AtomicU64,
    /// Total processing time in microseconds
    processing_time_us: AtomicU64,
    /// Last message timestamp
    last_message_time: RwLock<Option<Instant>>,
    /// Bytes received
    bytes_received: AtomicU64,
}

impl ConsumerMetrics {
    pub fn new(name: &str, topic: &str) -> Self {
        Self {
            _name: name.to_string(),
            _topic: topic.to_string(),
            messages_received: AtomicU64::new(0),
            messages_processed: AtomicU64::new(0),
            messages_failed: AtomicU64::new(0),
            processing_time_us: AtomicU64::new(0),
            last_message_time: RwLock::new(None),
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
}

/// Aggregated metrics for all Kafka consumers
pub struct KafkaMetricsRegistry {
    consumers: RwLock<HashMap<String, Arc<ConsumerMetrics>>>,
    start_time: Instant,
}

impl KafkaMetricsRegistry {
    pub fn new() -> Self {
        Self {
            consumers: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// Get all metrics as JSON
    pub async fn get_all_metrics(&self) -> serde_json::Value {
        let uptime = self.start_time.elapsed();
        serde_json::json!({
            "global": {
                "uptime_secs": uptime.as_secs(),
            },
            "consumers": [],
        })
    }

    /// Get metrics summary for health check
    pub async fn health_summary(&self) -> KafkaHealthSummary {
        let consumers = self.consumers.read().await;
        let unhealthy_consumers = Vec::new();

        KafkaHealthSummary {
            healthy: unhealthy_consumers.is_empty(),
            total_consumers: consumers.len(),
            unhealthy_consumers,
            total_lag: 0,
            global_success_rate: 1.0,
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
