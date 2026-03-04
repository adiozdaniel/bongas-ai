//! Phase 11: Ingestion Manager
//!
//! Orchestrates the activity pipeline, connecting various sources (Kafka, API) 
//! to the activity processor and monitoring metrics.

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::db::ResilientPool;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::resilience::ResilienceMetricsCollector;

use crate::ingestion::{ActivitySource, UserActivity};
use crate::ingestion::ActivityProcessor;
use crate::ingestion::{IngestionMetrics, IngestionHealth};
use crate::ingestion::recovery::kafka::service::KafkaSource;
use crate::ingestion::recovery::api::service::ApiSource;
use crate::ingestion::recovery::clickhouse::service::ClickHouseSource;
use crate::engine::governance::orchestration::manager::service::PagesManager;

/// Channel buffer size for the activity pipeline.
const ACTIVITY_CHANNEL_BUFFER: usize = 10_000;

/// Manages all activity sources and the processor.
pub struct IngestionManager {
    _processor: Arc<ActivityProcessor>,
    sources: Vec<Arc<dyn ActivitySource>>,
    api_source: Arc<ApiSource>,
    _metrics: Arc<IngestionMetrics>,
    _sender: mpsc::Sender<UserActivity>,
    worker_handle: Option<JoinHandle<()>>,
}

impl IngestionManager {
    pub async fn bootstrap(
        pool: Arc<ResilientPool>,
        clickhouse: Option<Arc<clickhouse::Client>>,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        resilience_metrics: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
        pages_manager: Arc<PagesManager>,
        kafka_brokers: String,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(ACTIVITY_CHANNEL_BUFFER);
        
        let api_source = Arc::new(ApiSource::new());
        let mut sources: Vec<Arc<dyn ActivitySource>> = vec![
            api_source.clone() as Arc<dyn ActivitySource>,
        ];

        // Add Kafka if configured
        if !kafka_brokers.is_empty() {
            let kafka_config = crate::ingestion::recovery::kafka::service::KafkaSourceConfig {
                brokers: kafka_brokers,
                playback_topic: "user-activities-playback".into(),
                reaction_topic: "user-activities-reaction".into(),
                profile_topic: "user-activities-profile".into(),
                notification_topic: "user-activities-notification".into(),
                group_id: "bongas-ingestion".to_string(),
            };
            let kafka = KafkaSource::new(kafka_config, breaker_registry.clone());
            sources.push(Arc::new(kafka));
        }

        // Add ClickHouse if configured
        if let Some(ref ch) = clickhouse {
            let ch_config = crate::ingestion::recovery::clickhouse::service::ClickHouseSourceConfig {
                poll_interval_secs: 60,
                batch_size: 1000,
            };
            sources.push(Arc::new(ClickHouseSource::new(ch_config, (**ch).clone(), breaker_registry.clone())));
        }

        let metrics = Arc::new(IngestionMetrics::new(sources.clone()));

        let processor = Arc::new(ActivityProcessor::new(
            Arc::new(crate::db::InteractionRepository::new(pool.clone(), resilience_metrics.clone())),
            pool.clone(),
            clickhouse.clone(),
            staleness_engine,
            pages_manager,
            resilience_metrics.clone(),
            50, // Max concurrent processing tasks
        ));

        let worker_rx = rx;
        let processor_clone = processor.clone();
        let handle = tokio::spawn(async move {
            processor_clone.start(worker_rx).await;
        });

        Ok(Self {
            _processor: processor,
            sources,
            api_source,
            _metrics: metrics,
            _sender: tx,
            worker_handle: Some(handle),
        })
    }

    pub fn api_source(&self) -> Arc<ApiSource> {
        self.api_source.clone()
    }

    pub async fn health(&self) -> IngestionHealth {
        let mut source_healths = Vec::new();
        for source in &self.sources {
            source_healths.push(source.health().await);
        }

        IngestionHealth {
            healthy: self.worker_handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false),
            total_sources: self.sources.len(),
            active_sources: self.sources.len(),
            degraded_sources: source_healths.iter().filter(|s| !s.healthy).map(|s| s.source_name.clone()).collect(),
            total_messages_ingested: source_healths.iter().map(|s| s.messages_ingested).sum(),
            total_errors: source_healths.iter().map(|s| s.errors).sum(),
            sources: source_healths,
        }
    }

    pub async fn broadcast_recommendations(&self, _user_id: i32, _profile_id: Option<String>, _scenario: String, _items: Vec<i32>) {
        // Implementation for ecosystem synergy (Phase 14)
    }
}
