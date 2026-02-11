use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdateEvent {
    pub user_id: i32,
    pub update_type: String,  // "preferences", "settings", etc.
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ProfileConsumer {
    consumer: StreamConsumer,
    db_pool: Arc<PgPool>,
}

impl ProfileConsumer {
    pub fn new(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
        db_pool: Arc<PgPool>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[topic])?;

        Ok(Self {
            consumer,
            db_pool,
        })
    }

    pub async fn start(self: Arc<Self>) {
        info!("Starting profile consumer");

        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        match self.process_message(payload).await {
                            Ok(_) => {
                                info!("Processed profile update");
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to process profile update");
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
        let event: ProfileUpdateEvent = serde_json::from_slice(payload)?;

        info!(
            user_id = event.user_id,
            update_type = event.update_type,
            "Processing profile update"
        );

        // Update user preferences in database
        // (Implementation depends on specific update_type)

        Ok(())
    }
}