//! Kafka-specific instrumentation.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, 
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

pub struct KafkaMetrics {
    pub messages_sent: IntCounterVec,
    pub messages_received: IntCounterVec,
    pub produce_latency: HistogramVec,
}

impl KafkaMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            messages_sent: register_int_counter_vec_with_registry!(
                opts!("kafka_messages_sent_total", "Total messages produced"),
                &[labels::TOPIC],
                registry
            )?,
            messages_received: register_int_counter_vec_with_registry!(
                opts!("kafka_messages_received_total", "Total messages consumed"),
                &[labels::TOPIC, labels::GROUP_ID],
                registry
            )?,
            produce_latency: register_histogram_vec_with_registry!(
                format!("{}_kafka_produce_duration_seconds", NAMESPACE),
                "Time taken to produce to Kafka",
                &[labels::TOPIC],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
