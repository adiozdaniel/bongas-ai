use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::engine::staleness_engine::StalenessEngine;
use super::playback_consumer::PlaybackConsumer;
use super::reaction_consumer::ReactionConsumer;
use super::profile_consumer::ProfileConsumer;
use super::notification_consumer::NotificationConsumer;

pub struct KafkaConsumerManager {
    handles: Vec<JoinHandle<()>>,
}

impl KafkaConsumerManager {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn start_all(
        &mut self,
        kafka_brokers: &str,
        db_pool: Arc<PgPool>,
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
    ) -> Result<()> {
        info!("Starting all Kafka consumers");

        // Start playback consumer
        let playback = Arc::new(PlaybackConsumer::new(
            kafka_brokers,
            "bongas-playback-consumer",
            "playback.sessions",
            resilient_pool.clone(),
            metrics_collector.clone(),
            staleness_engine.clone(),
        )?);
        self.handles.push(tokio::spawn(async move {
            playback.start().await;
        }));

        // Start reaction consumer
        let reaction = Arc::new(ReactionConsumer::new(
            kafka_brokers,
            "bongas-reaction-consumer",
            "user.reactions",
            resilient_pool.clone(),
            metrics_collector.clone(),
            staleness_engine.clone(),
        )?);
        self.handles.push(tokio::spawn(async move {
            reaction.start().await;
        }));

        // Start profile consumer
        let profile = Arc::new(ProfileConsumer::new(
            kafka_brokers,
            "bongas-profile-consumer",
            "user.profiles",
            db_pool.clone(),
            staleness_engine.clone(),
        )?);
        self.handles.push(tokio::spawn(async move {
            profile.start().await;
        }));

        // Start notification consumer
        let notification = Arc::new(NotificationConsumer::new(
            kafka_brokers,
            "bongas-notification-consumer",
            "notifications",
            staleness_engine.clone(),
        )?);
        self.handles.push(tokio::spawn(async move {
            notification.start().await;
        }));

        info!("All Kafka consumers started");
        Ok(())
    }

    pub async fn shutdown(self) {
        info!("Shutting down Kafka consumers");
        for handle in self.handles {
            handle.abort();
        }
    }
}