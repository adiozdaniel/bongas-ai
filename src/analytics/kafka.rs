//! Kafka-specific instrumentation.
//!
//! Provides comprehensive Prometheus metrics for monitoring Kafka consumer operations,
//! consumer group coordination, message processing, dead letter queue handling,
//! circuit breaker resilience patterns, and consumer health status. Enables
//! observability of streaming data pipeline performance and reliability.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for Kafka consumer operations and resilience patterns.
///
/// Maintains comprehensive metric vectors for:
/// * Message ingestion and processing throughput
/// * Consumer group coordination and partition assignment
/// * Consumer lag monitoring and offset management
/// * Dead letter queue error handling and retry policies
/// * Circuit breaker state transitions for fault tolerance
/// * Consumer health checking and availability
pub struct KafkaMetrics {
    /// Total messages received from topics, labeled by topic and consumer group.
    pub consumer_messages_received: IntCounterVec,
    /// Successfully processed messages, labeled by topic and consumer group.
    pub consumer_messages_processed: IntCounterVec,
    /// Failed message processing attempts, labeled by topic and consumer group.
    pub consumer_messages_failed: IntCounterVec,
    /// Message processing latency distribution, labeled by topic and consumer group.
    pub consumer_processing_latency: HistogramVec,

    /// Consumer group rebalance events, labeled by group ID.
    pub consumer_group_rebalances: IntCounterVec,
    /// Partition assignment events during rebalances, labeled by group ID.
    pub consumer_group_assignments: IntCounterVec,
    /// Duration of consumer group rebalancing operations, labeled by group ID.
    pub consumer_group_rebalance_latency: HistogramVec,

    /// Consumer lag in messages behind the latest offset, labeled by topic and group.
    pub consumer_lag: HistogramVec,
    /// Successful offset commit operations, labeled by topic and group.
    pub consumer_offset_committed: IntCounterVec,
    /// Failed offset commit attempts, labeled by topic and group.
    pub consumer_offset_commit_failures: IntCounterVec,

    /// Messages routed to dead letter queue, labeled by topic and error type.
    pub dlq_messages: IntCounterVec,
    /// Retry attempts for DLQ messages, labeled by topic and attempt number.
    pub dlq_retries: IntCounterVec,
    /// Permanent failures after retry exhaustion, labeled by topic and error type.
    pub dlq_failures: IntCounterVec,
    /// DLQ message processing latency, labeled by topic.
    pub dlq_processing_latency: HistogramVec,

    /// Circuit breaker state transitions, labeled by topic and state change.
    pub circuit_breaker_state_changes: IntCounterVec,
    /// Circuit breaker open events, labeled by topic.
    pub circuit_breaker_open_events: IntCounterVec,
    /// Circuit breaker half-open probe events, labeled by topic.
    pub circuit_breaker_half_open_events: IntCounterVec,
    /// Circuit breaker closed events, labeled by topic.
    pub circuit_breaker_closed_events: IntCounterVec,

    /// Consumer health check executions, labeled by topic and group.
    pub consumer_health_checks: IntCounterVec,
    /// Failed health check attempts, labeled by topic and group.
    pub consumer_health_failures: IntCounterVec,
    /// Health check latency distribution, labeled by topic and group.
    pub consumer_health_latency: HistogramVec,
}

impl KafkaMetrics {
    /// Creates and registers all Kafka consumer metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Consumer operation metrics
            consumer_messages_received: register_int_counter_vec_with_registry!(
                opts!("consumer_messages_received_total", "Total messages received by consumer"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_messages_processed: register_int_counter_vec_with_registry!(
                opts!("consumer_messages_processed_total", "Total messages processed by consumer"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_messages_failed: register_int_counter_vec_with_registry!(
                opts!("consumer_messages_failed_total", "Total messages failed by consumer"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_processing_latency: register_histogram_vec_with_registry!(
                format!("{}_consumer_processing_duration_seconds", NAMESPACE),
                "Consumer message processing latency",
                &[labels::TOPIC, labels::GROUP_ID],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Consumer group metrics
            consumer_group_rebalances: register_int_counter_vec_with_registry!(
                opts!("consumer_group_rebalances_total", "Total consumer group rebalances"),
                &[labels::GROUP_ID],
                registry
            )?,
            consumer_group_assignments: register_int_counter_vec_with_registry!(
                opts!("consumer_group_assignments_total", "Total consumer group partition assignments"),
                &[labels::GROUP_ID],
                registry
            )?,
            consumer_group_rebalance_latency: register_histogram_vec_with_registry!(
                format!("{}_consumer_group_rebalance_duration_seconds", NAMESPACE),
                "Consumer group rebalance latency",
                &[labels::GROUP_ID],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Consumer lag metrics
            consumer_lag: register_histogram_vec_with_registry!(
                format!("{}_consumer_lag", NAMESPACE),
                "Consumer lag in messages",
                &[labels::TOPIC, labels::GROUP_ID],
                vec![0.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0],
                registry
            )?,
            consumer_offset_committed: register_int_counter_vec_with_registry!(
                opts!("consumer_offset_commits_total", "Total consumer offset commits"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_offset_commit_failures: register_int_counter_vec_with_registry!(
                opts!("consumer_offset_commit_failures_total", "Total consumer offset commit failures"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,

            // Dead letter queue metrics
            dlq_messages: register_int_counter_vec_with_registry!(
                opts!("dlq_messages_total", "Total messages sent to dead letter queue"),
                &[labels::TOPIC, "error_type"],
                registry
            )?,
            dlq_retries: register_int_counter_vec_with_registry!(
                opts!("dlq_retries_total", "Total dead letter queue retry attempts"),
                &[labels::TOPIC, "retry_attempt"],
                registry
            )?,
            dlq_failures: register_int_counter_vec_with_registry!(
                opts!("dlq_failures_total", "Total dead letter queue permanent failures"),
                &[labels::TOPIC, "error_type"],
                registry
            )?,
            dlq_processing_latency: register_histogram_vec_with_registry!(
                format!("{}_dlq_processing_duration_seconds", NAMESPACE),
                "Dead letter queue message processing latency",
                &[labels::TOPIC],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Circuit breaker metrics
            circuit_breaker_state_changes: register_int_counter_vec_with_registry!(
                opts!("circuit_breaker_state_changes_total", "Total circuit breaker state changes"),
                &[labels::TOPIC, "from_state", "to_state"],
                registry
            )?,
            circuit_breaker_open_events: register_int_counter_vec_with_registry!(
                opts!("circuit_breaker_open_events_total", "Total circuit breaker open events"),
                &[labels::TOPIC],
                registry
            )?,
            circuit_breaker_half_open_events: register_int_counter_vec_with_registry!(
                opts!("circuit_breaker_half_open_events_total", "Total circuit breaker half-open events"),
                &[labels::TOPIC],
                registry
            )?,
            circuit_breaker_closed_events: register_int_counter_vec_with_registry!(
                opts!("circuit_breaker_closed_events_total", "Total circuit breaker closed events"),
                &[labels::TOPIC],
                registry
            )?,

            // Consumer health metrics
            consumer_health_checks: register_int_counter_vec_with_registry!(
                opts!("consumer_health_checks_total", "Total consumer health checks"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_health_failures: register_int_counter_vec_with_registry!(
                opts!("consumer_health_failures_total", "Total consumer health check failures"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            consumer_health_latency: register_histogram_vec_with_registry!(
                format!("{}_consumer_health_check_duration_seconds", NAMESPACE),
                "Consumer health check latency",
                &[labels::TOPIC, labels::GROUP_ID],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
