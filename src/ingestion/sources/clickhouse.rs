//! ClickHouse polling source — active ingestion from analytics database.
//!
//! Polls ClickHouse for user interactions on an interval. Runs alongside
//! Kafka and API sources as an equal peer. If disabled via config, it
//! simply doesn't start.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{info, error, debug, warn};

use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId, CircuitBreakerConfig, CircuitState};
use super::super::types::{ActivitySource, SourceHealth, UserActivity};

/// Configuration for the ClickHouse polling source.
#[derive(Debug, Clone)]
pub struct ClickHouseSourceConfig {
    /// ClickHouse connection URL.
    pub url: String,
    /// How often to poll (seconds).
    pub poll_interval_secs: u64,
    /// Whether this source is enabled.
    pub enabled: bool,
}

impl Default for ClickHouseSourceConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            poll_interval_secs: 60,
            enabled: true,
        }
    }
}

/// ClickHouse-based activity source for backfill/fallback.
pub struct ClickHouseSource {
    config: ClickHouseSourceConfig,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
    /// Tracks the last processed event timestamp to avoid re-processing.
    last_checkpoint: tokio::sync::RwLock<chrono::DateTime<chrono::Utc>>,
}

impl ClickHouseSource {
    pub fn new(
        config: ClickHouseSourceConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        Self {
            config,
            circuit_breaker_registry,
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            last_checkpoint: tokio::sync::RwLock::new(chrono::Utc::now()),
        }
    }

    fn breaker_id() -> CircuitBreakerId {
        CircuitBreakerId::new("ingestion:clickhouse")
    }

    /// Poll ClickHouse for recent interactions.
    async fn poll_interactions(&self, sender: &mpsc::Sender<UserActivity>) -> Result<u64> {
        let checkpoint = *self.last_checkpoint.read().await;
        let breaker = self.circuit_breaker_registry.get_or_create(
            Self::breaker_id(),
            CircuitBreakerConfig::default(),
        );

        if breaker.health().state == CircuitState::Open {
            warn!("ClickHouse circuit breaker open, skipping poll");
            return Ok(0);
        }

        // TODO: Replace with actual ClickHouse client query.
        // The query should look like:
        //
        //   SELECT user_id, item_id, event_type, watch_duration, watch_percentage,
        //          completed, reaction_type, event_time
        //   FROM user_events
        //   WHERE event_time > {checkpoint}
        //   ORDER BY event_time ASC
        //   LIMIT 1000
        //
        // For now, this is a structured placeholder that logs the intent.
        debug!(
            checkpoint = %checkpoint,
            url = %self.config.url,
            "Would poll ClickHouse for events since checkpoint"
        );

        // Update checkpoint to now
        *self.last_checkpoint.write().await = chrono::Utc::now();

        breaker.record_success();
        Ok(0)
    }
}

#[async_trait::async_trait]
impl ActivitySource for ClickHouseSource {
    fn name(&self) -> &str {
        "clickhouse"
    }

    async fn start(&self, sender: mpsc::Sender<UserActivity>) -> Result<()> {
        info!(
            poll_interval_secs = self.config.poll_interval_secs,
            "ClickHouse polling source started"
        );

        let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));

        loop {
            ticker.tick().await;

            match self.poll_interactions(&sender).await {
                Ok(count) => {
                    if count > 0 {
                        self.messages_ingested.fetch_add(count, Ordering::Relaxed);
                        info!(count, "ClickHouse backfill delivered activities");
                    }
                }
                Err(e) => {
                    error!(error = %e, "ClickHouse poll failed");
                    self.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    async fn health(&self) -> SourceHealth {
        let breaker = self.circuit_breaker_registry.get_or_create(
            Self::breaker_id(),
            CircuitBreakerConfig::default(),
        );
        let health = breaker.health();

        SourceHealth {
            source_name: "clickhouse".to_string(),
            healthy: health.state != CircuitState::Open,
            messages_ingested: self.messages_ingested.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            circuit_state: format!("{:?}", health.state),
            last_activity: None,
        }
    }
}
