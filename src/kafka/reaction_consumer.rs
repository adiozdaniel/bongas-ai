use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};
use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::engine::staleness_engine::{StalenessEngine, UserEvent};
use crate::db::repositories::interaction_repository::InteractionRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEvent {
    pub user_id: i32,
    pub item_id: i32,
    pub reaction_type: String,  // "like" or "dislike"
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ReactionConsumer {
    consumer: StreamConsumer,
    interaction_repo: Arc<InteractionRepository>,
    staleness_engine: Arc<StalenessEngine>,
}

impl ReactionConsumer {
    pub fn new(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[topic])?;

        let interaction_repo = Arc::new(InteractionRepository::new(resilient_pool, metrics_collector));

        Ok(Self {
            consumer,
            interaction_repo,
            staleness_engine,
        })
    }

    pub async fn start(self: Arc<Self>) {
        info!("Starting reaction consumer");

        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        match self.process_message(payload).await {
                            Ok(_) => {
                                info!("Processed reaction message");
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to process reaction");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = ?e, "Kafka consumer error");
                }
            }
        }
    }

    async fn process_message(&self, payload: &[u8]) -> Result<()> {
        let event: ReactionEvent = serde_json::from_slice(payload)?;

        let rating = match event.reaction_type.as_str() {
            "like" => 5.0,
            "dislike" => 1.0,
            _ => 3.0,
        };

        info!(
            user_id = event.user_id,
            item_id = event.item_id,
            reaction_type = event.reaction_type,
            "Processing explicit feedback"
        );

        // Store explicit rating
        self.interaction_repo
            .create_explicit_rating(event.user_id, event.item_id, rating)
            .await?;

        // Invalidate personalized caches
        let staleness_event = UserEvent::ExplicitFeedback {
            user_id: event.user_id,
            item_id: event.item_id,
            rating,
        };

        self.staleness_engine.process_event(&staleness_event).await?;

        Ok(())
    }
}