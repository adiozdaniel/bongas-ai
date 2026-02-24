//! Ingestion manager implementation.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::db::ResilientPool;
use crate::engine::staleness_engine::StalenessEngine;
use crate::resilience::ResilienceMetricsCollector;

use crate::ingestion::types::{ActivitySource, UserActivity};
use crate::ingestion::processor::ActivityProcessor;
use crate::ingestion::metrics::{IngestionMetrics, IngestionHealth};
use crate::ingestion::sources::{KafkaSource, ApiSource, ClickHouseSource};
use crate::ingestion::producer::RecommendationProducer;

/// Channel buffer size for the activity pipeline.
const ACTIVITY_CHANNEL_BUFFER: usize = 10_000;

/// Manages all activity sources and the processor.
pub struct IngestionManager {
    pub(crate) config: crate::config::IngestionConfig,
    pub(crate) resilient_pool: Arc<ResilientPool>,
    pub(crate) metrics_collector: Arc<ResilienceMetricsCollector>,
    pub(crate) staleness_engine: Arc<StalenessEngine>,
    pub(crate) circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    pub(crate) handles: Vec<JoinHandle<()>>,
    pub(crate) api_source: Arc<ApiSource>,
    pub(crate) metrics: Arc<IngestionMetrics>,
    pub(crate) clickhouse_client: Option<Arc<clickhouse::Client>>,
    pub(crate) recommendation_producer: Arc<RecommendationProducer>,
}

impl IngestionManager {
    /// Create a new IngestionManager and start all sources.
    pub fn new(
        config: crate::config::IngestionConfig,
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
        ingestion_metrics: Arc<IngestionMetrics>,
        clickhouse_client: Option<Arc<clickhouse::Client>>,
    ) -> Self {
        let recommendation_producer = Arc::new(RecommendationProducer::new(&config.kafka));
        
        Self {
            config,
            resilient_pool,
            metrics_collector,
            staleness_engine,
            circuit_breaker_registry,
            handles: Vec::new(),
            api_source: Arc::new(ApiSource::new()),
            metrics: ingestion_metrics,
            clickhouse_client,
            recommendation_producer,
        }
    }

    /// Start the ingestion pipeline with all configured sources.
    pub async fn start(&mut self) -> Result<()> {
        let (sender, receiver) = mpsc::channel::<UserActivity>(ACTIVITY_CHANNEL_BUFFER);

        let mut all_sources: Vec<Arc<dyn ActivitySource>> = Vec::new();

        if !self.config.kafka.brokers.is_empty() {
            let kafka = Arc::new(KafkaSource::new(
                self.config.kafka.clone().into(),
                self.circuit_breaker_registry.clone(),
            ));
            all_sources.push(kafka.clone());

            let tx = sender.clone();
            let kafka_for_task = kafka.clone();
            self.handles.push(tokio::spawn(async move {
                if let Err(e) = kafka_for_task.start(tx).await {
                    warn!(error = %e, "Kafka source exited with error");
                }
            }));

            info!("Kafka activity source enabled (auto-detected brokers)");
        }

        {
            all_sources.push(self.api_source.clone());

            let tx = sender.clone();
            let api = self.api_source.clone();
            self.handles.push(tokio::spawn(async move {
                if let Err(e) = api.start(tx).await {
                    warn!(error = %e, "API source exited with error");
                }
            }));

            info!("API activity source enabled");
        }

        if let Some(ref client) = self.clickhouse_client {
            let clickhouse = Arc::new(ClickHouseSource::new(
                self.config.clickhouse.clone().into(),
                (**client).clone(),
                self.circuit_breaker_registry.clone(),
            ));
            all_sources.push(clickhouse.clone());

            let tx = sender.clone();
            let ch_for_task = clickhouse.clone();
            self.handles.push(tokio::spawn(async move {
                if let Err(e) = ch_for_task.start(tx).await {
                    warn!(error = %e, "ClickHouse source exited with error");
                }
            }));

            info!("ClickHouse polling enabled (auto-detected client)");
        }

        let processor = Arc::new(ActivityProcessor::new(
            self.resilient_pool.clone(),
            self.metrics_collector.clone(),
            self.staleness_engine.clone(),
            self.clickhouse_client.clone(),
        ));

        self.handles.push(tokio::spawn(async move {
            processor.run(receiver).await;
        }));

        self.metrics.update_sources(all_sources);

        info!(
            source_count = self.handles.len() - 1,
            "Ingestion pipeline started"
        );

        Ok(())
    }

    /// Get a reference to the API source for handler integration.
    pub fn api_source(&self) -> Arc<ApiSource> {
        self.api_source.clone()
    }

    /// Broadcast recommendation results to the ecosystem.
    pub async fn broadcast_recommendations(&self, user_id: i32, profile_id: Option<String>, scenario: String, item_ids: Vec<i32>) {
        self.recommendation_producer.broadcast_results(user_id, profile_id, scenario, item_ids).await;
    }

    /// Get aggregated health across all sources.
    pub async fn health(&self) -> IngestionHealth {
        self.metrics.health().await
    }

    /// Shutdown all sources and the processor.
    pub async fn shutdown(&mut self) {
        info!("Shutting down ingestion pipeline");
        for handle in self.handles.drain(..) {
            handle.abort();
        }
        info!("Ingestion pipeline shut down");
    }
}
