//! Phase 11: Ingestion Manager
//!
//! Orchestrates the activity pipeline, connecting various sources (Kafka, API, ClickHouse)
//! to the centralized ActivityProcessor.

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::ingestion::types::{UserActivity, ActivitySource};
use crate::ingestion::recovery::kafka::service::KafkaSource;
use crate::ingestion::recovery::clickhouse::service::ClickHouseSource;
use crate::ingestion::processing::processor::service::ActivityProcessor;
use crate::ingestion::broadcast::metrics::service::IngestionMetrics;
use crate::ingestion::IngestionHealth;
use crate::ingestion::recovery::api::service::ApiSource;
use crate::db::ResilientPool;
use crate::engine::intelligence::pillar::service::IntelligencePillar;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::resilience::ResilienceMetricsCollector;
use crate::engine::intelligence::monitoring::staleness_engine::service::StalenessEngine;
use crate::engine::governance::orchestration::manager::service::PagesManager;
use crate::config::KafkaConfig;
use crate::cache::CacheManager;

const ACTIVITY_CHANNEL_BUFFER: usize = 10000;

/// The central entry point for all activity ingestion in Bongas-AI.
pub struct IngestionManager {
    _processor: Arc<ActivityProcessor>,
    _sources: Vec<Arc<dyn ActivitySource>>,
    api_source: Arc<ApiSource>,
    _metrics: Arc<IngestionMetrics>,
    _sender: mpsc::Sender<UserActivity>,
    _worker_handle: Option<JoinHandle<()>>,
}

impl IngestionManager {
    pub async fn bootstrap(
        pool: Arc<ResilientPool>,
        intelligence: Arc<IntelligencePillar>,
        cache_manager: Arc<CacheManager>,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        resilience_metrics: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
        pages_manager: Arc<PagesManager>,
        kafka_config: KafkaConfig,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(ACTIVITY_CHANNEL_BUFFER);
        
        let api_source = Arc::new(ApiSource::new());
        let mut sources: Vec<Arc<dyn ActivitySource>> = vec![
            api_source.clone() as Arc<dyn ActivitySource>,
        ];

        // Add Kafka if configured
        if kafka_config.enabled {
            let kafka_source_config = crate::ingestion::recovery::kafka::service::KafkaSourceConfig::from(kafka_config);
            let kafka = KafkaSource::new(kafka_source_config, breaker_registry.clone());
            sources.push(Arc::new(kafka));
        }

        // Add ClickHouse Source if configured
        if let Some(ch) = intelligence.clickhouse_client() {
            let ch_config = crate::ingestion::recovery::clickhouse::service::ClickHouseSourceConfig {
                poll_interval_secs: 60,
                batch_size: 1000,
            };
            sources.push(Arc::new(ClickHouseSource::new(ch_config, ch, breaker_registry.clone())));
        }

        let metrics = Arc::new(IngestionMetrics::new(sources.clone()));

        let processor = Arc::new(ActivityProcessor::new(
            Arc::new(crate::db::InteractionRepository::new(pool.clone(), resilience_metrics.clone())),
            pool.clone(),
            cache_manager,
            intelligence,
            staleness_engine,
            pages_manager,
            resilience_metrics.clone(),
        ));

        let worker_rx = rx;
        let processor_clone = processor.clone();
        let handle = tokio::spawn(async move {
            processor_clone.start(worker_rx).await;
        });

        Ok(Self {
            _processor: processor,
            _sources: sources,
            api_source,
            _metrics: metrics,
            _sender: tx,
            _worker_handle: Some(handle),
        })
    }

    pub fn api_source(&self) -> Arc<ApiSource> {
        self.api_source.clone()
    }

    /// Get aggregated ingestion health.
    pub async fn health(&self) -> IngestionHealth {
        self._metrics.health().await
    }

    /// Broadcast a recommendation event to the ingestion pipeline (internal sink).
    pub async fn broadcast_recommendations(&self, user_id: i32, profile_id: Option<String>, scenario: String, item_ids: Vec<i32>) {
        for id in item_ids {
            let activity = UserActivity::Impression {
                user_id,
                profile_id: profile_id.clone(),
                item_id: id,
                visitor_id: None,
                device_hash: None,
                device_type: None,
                scenario_slug: Some(scenario.clone()),
                timestamp: chrono::Utc::now(),
            };
            let _ = self._sender.send(activity).await;
        }
    }
}
