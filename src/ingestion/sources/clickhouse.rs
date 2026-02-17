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
use tracing::{info, error, warn};
use async_trait::async_trait;

use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId, CircuitBreakerConfig, CircuitState};
use super::super::types::{ActivitySource, SourceHealth, UserActivity};
use crate::config::types::ingestion as config_ingestion;

/// Configuration for the ClickHouse polling source.
#[derive(Debug, Clone)]
pub struct ClickHouseSourceConfig {
    /// How often to poll (seconds).
    pub poll_interval_secs: u64,
    /// Whether this source is enabled.
    pub enabled: bool,
    /// Max events to fetch per poll.
    pub batch_size: u32,
}

impl Default for ClickHouseSourceConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            enabled: true,
            batch_size: 1000,
        }
    }
}

impl From<config_ingestion::ClickHouseSourceConfig> for ClickHouseSourceConfig {
    fn from(config: config_ingestion::ClickHouseSourceConfig) -> Self {
        ClickHouseSourceConfig {
            poll_interval_secs: config.poll_interval_secs,
            enabled: config.enabled,
            batch_size: config.batch_size as u32,
        }
    }
}

#[derive(serde::Deserialize, clickhouse::Row)]
struct ClickHouseEvent {
    pub user_id: i32,
    pub item_id: i32,
    pub event_type: String,
    pub watch_duration: i32,
    pub watch_percentage: f32,
    pub completed: bool,
    pub reaction_type: String,
    pub event_time: u32,
}

/// ClickHouse-based activity source for backfill/fallback.
pub struct ClickHouseSource {
    config: ClickHouseSourceConfig,
    client: clickhouse::Client,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    messages_ingested: AtomicU64,
    errors: AtomicU64,
    /// Tracks the last processed event timestamp to avoid re-processing.
    last_checkpoint: tokio::sync::RwLock<chrono::DateTime<chrono::Utc>>,
}

impl ClickHouseSource {
    pub fn new(
        config: ClickHouseSourceConfig,
        client: clickhouse::Client,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        Self {
            config,
            client,
            circuit_breaker_registry,
            messages_ingested: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            last_checkpoint: tokio::sync::RwLock::new(chrono::Utc::now() - chrono::Duration::try_minutes(5).unwrap()),
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

        if breaker.current_state() == CircuitState::Open {
            warn!("ClickHouse circuit breaker open, skipping poll");
            return Ok(0);
        }

        let query = "SELECT user_id, item_id, event_type, watch_duration, watch_percentage, \
                     completed, reaction_type, toUnixTimestamp(event_time) as event_time \
                     FROM user_events \
                     WHERE event_time > toDateTime(?) \
                     ORDER BY event_time ASC \
                     LIMIT ?";

        let rows: Vec<ClickHouseEvent> = self.client
            .query(query)
            .bind(checkpoint.timestamp() as u32)
            .bind(self.config.batch_size)
            .fetch_all()
            .await?;

        let mut count = 0;
        let mut latest_timestamp = checkpoint;

        for row in rows {
            let event_time = chrono::DateTime::from_timestamp(row.event_time as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now());

            if event_time > latest_timestamp {
                latest_timestamp = event_time;
            }

            let activity = match row.event_type.as_str() {
                "playback" => UserActivity::Playback {
                    user_id: row.user_id,
                    item_id: row.item_id,
                    session_id: "clickhouse_backfill".to_string(),
                    watch_duration_seconds: row.watch_duration,
                    total_duration_seconds: if row.watch_percentage > 0.0 {
                        (row.watch_duration as f32 / row.watch_percentage) as i32
                    } else {
                        row.watch_duration
                    },
                    watch_percentage: row.watch_percentage,
                    completed: row.completed,
                    scenario_slug: None,
                    timestamp: event_time,
                },
                "reaction" => UserActivity::Reaction {
                    user_id: row.user_id,
                    item_id: row.item_id,
                    reaction_type: row.reaction_type,
                    scenario_slug: None,
                    timestamp: event_time,
                },
                "click" => UserActivity::Click {
                    user_id: row.user_id,
                    item_id: row.item_id,
                    scenario_slug: None,
                    timestamp: event_time,
                },
                "impression" => UserActivity::Impression {
                    user_id: row.user_id,
                    item_id: row.item_id,
                    scenario_slug: None,
                    timestamp: event_time,
                },
                _ => continue,
            };

            if sender.send(activity).await.is_ok() {
                count += 1;
            }
        }

        if count > 0 {
            *self.last_checkpoint.write().await = latest_timestamp;
        }

        Ok(count as u64)
    }
}

#[async_trait]
impl ActivitySource for ClickHouseSource {
    fn name(&self) -> &str {
        "clickhouse"
    }

    async fn start(&self, sender: mpsc::Sender<UserActivity>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

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
