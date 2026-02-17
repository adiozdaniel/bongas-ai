//! Phase 5: Ecosystem Synergy (The Intelligence Broadcaster)
//!
//! Synchronizes Bongas-AI predictions with other services (Catalog, Video)
//! via Kafka to enable pre-warming and side-loading.

use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::config::ClientConfig;
use std::time::Duration;
use tracing::{info, error};
use serde::Serialize;

use crate::config::KafkaConfig;

#[derive(Debug, Serialize)]
pub struct RecommendationSyncEvent {
    pub user_id: i32,
    pub scenario: String,
    pub item_ids: Vec<i32>,
    pub timestamp: u64,
}

pub struct RecommendationProducer {
    producer: FutureProducer,
    topic: String,
}

impl RecommendationProducer {
    pub fn new(config: &KafkaConfig) -> Self {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        Self {
            producer,
            topic: "recommendations.sync".to_string(),
        }
    }

    /// Asynchronously broadcast recommendation results to the ecosystem.
    pub async fn broadcast_results(
        &self,
        user_id: i32,
        scenario: String,
        item_ids: Vec<i32>,
    ) {
        let event = RecommendationSyncEvent {
            user_id,
            scenario,
            item_ids,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let payload = match serde_json::to_string(&event) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to serialize recommendation sync event: {}", e);
                return;
            }
        };

        let key = format!("user:{}", user_id);
        let record = FutureRecord::to(&self.topic)
            .payload(&payload)
            .key(&key);

        let result = self.producer.send(record, Duration::from_secs(0)).await;

        match result {
            Ok(_) => info!(user_id, scenario = %event.scenario, "Broadcasted recommendations to Kafka"),
            Err((e, _)) => error!("Failed to broadcast recommendations to Kafka: {:?}", e),
        }
    }
}
