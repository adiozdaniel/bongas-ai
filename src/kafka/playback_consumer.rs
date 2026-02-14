use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{info, error};
use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::engine::staleness_engine::{StalenessEngine, UserEvent};
use crate::db::repositories::interaction_repository::InteractionRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedPlaybackSession {
    pub user_id: i32,
    pub item_id: i32,
    pub session_id: String,
    pub watch_duration_seconds: i32,
    pub total_duration_seconds: i32,
    pub watch_percentage: f32,
    pub completed: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct PlaybackConsumer {
    consumer: StreamConsumer,
    interaction_repo: Arc<InteractionRepository>,
    staleness_engine: Arc<StalenessEngine>,
}

impl PlaybackConsumer {
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
            .set("session.timeout.ms", "30000")
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
        info!("Starting playback consumer");

        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        match self.process_message(payload).await {
                            Ok(_) => {
                                info!("Processed playback message successfully");
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to process playback message");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(error = ?e, "Kafka consumer error");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_message(&self, payload: &[u8]) -> Result<()> {
        let event: ProcessedPlaybackSession = serde_json::from_slice(payload)?;

        // Convert watch percentage to implicit rating (1-5 scale)
        let rating = match event.watch_percentage {
            p if p < 0.25 => 1.0,
            p if p < 0.50 => 2.0,
            p if p < 0.75 => 3.5,
            _ => 5.0,
        };

        // Bonus for completion
        let rating = if event.completed {
            let new_rating = rating + 0.5;
            if new_rating > 5.0 {
                5.0
            } else {
                new_rating
            }
        } else {
            rating
        };

        info!(
            user_id = event.user_id,
            item_id = event.item_id,
            watch_percentage = event.watch_percentage,
            rating = rating,
            "Generated implicit rating from playback"
        );

        // Store as implicit interaction
        self.interaction_repo
            .create_implicit_rating(
                event.user_id,
                event.item_id,
                rating,
                event.watch_duration_seconds,
            )
            .await?;

        // Invalidate cache if necessary
        let staleness_event = UserEvent::WatchEvent {
            user_id: event.user_id,
            item_id: event.item_id,
            completion_rate: event.watch_percentage,
        };

        self.staleness_engine.process_event(&staleness_event).await?;

        Ok(())
    }
}