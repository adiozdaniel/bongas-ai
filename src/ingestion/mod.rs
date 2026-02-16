//! Activity ingestion backbone — source-agnostic user activity pipeline.
//!
//! Three sources feed activities through one processor into the staleness engine:
//! - **Kafka** — real-time stream consumer
//! - **API** — direct endpoint for user-reaction calls
//! - **ClickHouse** — polling feadback
//!
//! All sources convert their native format into `UserActivity` and push into
//! a shared channel. The `ActivityProcessor` normalizes, writes to DB, and
//! feeds the staleness engine for cache invalidation.

pub mod types;
pub mod processor;
pub mod sources;
pub mod metrics;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::db::ResilientPool;
use crate::engine::staleness_engine::StalenessEngine;
use crate::resilience::ResilienceMetricsCollector;

use self::types::{ActivitySource, UserActivity};
use self::processor::ActivityProcessor;
use self::metrics::{IngestionMetrics, IngestionHealth};
use self::sources::{KafkaSource, ApiSource, ClickHouseSource};


/// Channel buffer size for the activity pipeline.
const ACTIVITY_CHANNEL_BUFFER: usize = 10_000;



/// Manages all activity sources and the processor.
///
/// Owns the lifecycle of the ingestion pipeline: start, health, shutdown.
pub struct IngestionManager {
    config: crate::config::IngestionConfig,
    resilient_pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
    staleness_engine: Arc<StalenessEngine>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    handles: Vec<JoinHandle<()>>,
    api_source: Arc<ApiSource>,
    metrics: Arc<IngestionMetrics>,
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
    ) -> Self {
        Self {
            config,
            resilient_pool,
            metrics_collector,
            staleness_engine,
            circuit_breaker_registry,
            handles: Vec::new(),
            api_source: Arc::new(ApiSource::new()),
            metrics: ingestion_metrics,
        }
    }

    /// Start the ingestion pipeline with all configured sources.
    pub async fn start(&mut self) -> Result<()> {
        let (sender, receiver) = mpsc::channel::<UserActivity>(ACTIVITY_CHANNEL_BUFFER);

        // ── Build sources ───────────────────────────────────────────────
        let mut all_sources: Vec<Arc<dyn ActivitySource>> = Vec::new();

        // 1. Kafka source (if brokers configured)
        if !self.config.kafka.brokers.is_empty() {
            let kafka = Arc::new(KafkaSource::new(
                self.config.kafka.clone().into(),
                self.circuit_breaker_registry.clone(),
            ));
            all_sources.push(kafka.clone());

            let tx = sender.clone();
            self.handles.push(tokio::spawn(async move {
                if let Err(e) = kafka.start(tx).await {
                    warn!(error = %e, "Kafka source exited with error");
                }
            }));

            info!("Kafka activity source enabled");
        }

        // 2. API source
        if self.config.api.enabled {
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

        // 3. ClickHouse polling source
        let clickhouse = Arc::new(ClickHouseSource::new(
            self.config.clickhouse.clone().into(),
            self.circuit_breaker_registry.clone(),
        ));
        all_sources.push(clickhouse.clone());

        let tx = sender.clone();
        self.handles.push(tokio::spawn(async move {
            if let Err(e) = clickhouse.start(tx).await {
                warn!(error = %e, "ClickHouse source exited with error");
            }
        }));

        info!("ClickHouse polling enabled");

        // ── Start processor ─────────────────────────────────────────────
        let processor = Arc::new(ActivityProcessor::new(
            self.resilient_pool.clone(),
            self.metrics_collector.clone(),
            self.staleness_engine.clone(),
        ));

        self.handles.push(tokio::spawn(async move {
            processor.run(receiver).await;
        }));

        // Update metrics with actual sources
        self.metrics.update_sources(all_sources);

        info!(
            source_count = self.handles.len() - 1,
            "Ingestion pipeline started"
        );

        Ok(())
    }

    /// Legacy start method that creates everything.
    pub async fn start_legacy(
        config: crate::config::types::ingestion::IngestionConfig,
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Result<Self> {
        let (sender, receiver) = mpsc::channel::<UserActivity>(ACTIVITY_CHANNEL_BUFFER);

        // ── Build sources ───────────────────────────────────────────────
        let mut all_sources: Vec<Arc<dyn ActivitySource>> = Vec::new();
        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        // 1. Kafka source (if brokers configured)
        if !config.kafka.brokers.is_empty() {
            let kafka = Arc::new(KafkaSource::new(
                config.kafka.clone().into(),
                circuit_breaker_registry.clone(),
            ));
            all_sources.push(kafka.clone());

            let tx = sender.clone();
            handles.push(tokio::spawn(async move {
                if let Err(e) = kafka.start(tx).await {
                    warn!(error = %e, "Kafka source exited with error");
                }
            }));

            info!("Kafka activity source enabled");
        } else {
            info!("Kafka activity source disabled (no brokers configured)");
        }

        // 2. API source (always created — handlers may call ingest())
        let api_source = Arc::new(ApiSource::new());
        if config.api.enabled {
            all_sources.push(api_source.clone());

            let tx = sender.clone();
            let api = api_source.clone();
            handles.push(tokio::spawn(async move {
                if let Err(e) = api.start(tx).await {
                    warn!(error = %e, "API source exited with error");
                }
            }));

            info!("API activity source enabled");
        }

        // 3. ClickHouse polling source
        let clickhouse = Arc::new(ClickHouseSource::new(
            config.clickhouse.clone().into(),
            circuit_breaker_registry.clone(),
        ));
        all_sources.push(clickhouse.clone());

        let tx = sender.clone();
        handles.push(tokio::spawn(async move {
            if let Err(e) = clickhouse.start(tx).await {
                warn!(error = %e, "ClickHouse source exited with error");
            }
        }));

        info!("ClickHouse polling enabled");

        // ── Start processor ─────────────────────────────────────────────
        let processor = Arc::new(ActivityProcessor::new(
            resilient_pool.clone(),
            metrics_collector.clone(),
            staleness_engine.clone(),
        ));

        handles.push(tokio::spawn(async move {
            processor.run(receiver).await;
        }));

        info!("Activity processor started");

        // ── Build metrics ───────────────────────────────────────────────
        let metrics = Arc::new(IngestionMetrics::new(all_sources));

        info!(
            source_count = handles.len() - 1, // subtract processor
            "Ingestion pipeline started"
        );

        Ok(Self {
            config,
            resilient_pool,
            metrics_collector,
            staleness_engine,
            circuit_breaker_registry,
            handles,
            api_source,
            metrics,
        })
    }

    /// Get a reference to the API source for handler integration.
    pub fn api_source(&self) -> Arc<ApiSource> {
        self.api_source.clone()
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
