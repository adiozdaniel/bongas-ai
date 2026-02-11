use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub user_id: i32,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct NotificationConsumer {
    consumer: StreamConsumer,
}

impl NotificationConsumer {
    pub fn new(
        kafka_brokers: &str,
        group_id: &str,
        topic: &str,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[topic])?;

        Ok(Self { consumer })
    }

    pub async fn start(self: Arc<Self>) {
        info!("Starting notification consumer");

        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        match self.process_message(payload).await {
                            Ok(_) => {
                                info!("Processed notification");
                            }
                            Err(e) => {
                                error!(error = ?e, "Failed to process notification");
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
        let event: NotificationEvent = serde_json::from_slice(payload)?;

        info!(
            user_id = event.user_id,
            notification_type = event.notification_type,
            "Processing notification"
        );

        // Send push notification via external service
        // (Implementation depends on notification provider)

        Ok(())
    }
}