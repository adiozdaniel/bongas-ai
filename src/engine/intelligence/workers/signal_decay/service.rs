use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};
use anyhow::Result;
use clickhouse::Client as ClickHouseClient;

/// 🪵 Signal Decay Engine: Prunes stale behavioral data.
/// 
/// Ensures infrastructure costs stay flat by "forgetting" irrelevant history
/// from ClickHouse automatically based on configured TTL.
pub struct SignalDecayWorker {
    clickhouse: ClickHouseClient,
    pulse_interval: Duration,
    retention_days: u32,
}

impl SignalDecayWorker {
    pub fn new(
        clickhouse: ClickHouseClient,
        pulse_interval: Duration,
        retention_days: u32,
    ) -> Self {
        Self {
            clickhouse,
            pulse_interval,
            retention_days,
        }
    }

    /// Start the background decay loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!(
            interval_hours = self.pulse_interval.as_secs() / 3600,
            retention_days = self.retention_days,
            "Signal Decay Engine started"
        );

        let mut ticker = interval(self.pulse_interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_decay_cycle().await {
                        error!(error = %e, "Signal decay cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Signal Decay Engine shutting down...");
                    break;
                }
            }
        }
    }

    async fn run_decay_cycle(&self) -> Result<()> {
        debug!("Running signal decay cycle...");

        // ClickHouse DELETE is an asynchronous mutation
        // We prune data older than retention_days
        let threshold_unix = (chrono::Utc::now() - chrono::Duration::days(self.retention_days as i64)).timestamp();

        let query = "ALTER TABLE user_interactions DELETE WHERE created_at < ?";

        self.clickhouse.query(query).bind(threshold_unix).execute().await?;

        info!(
            threshold_unix,
            "Dispatched decay mutation to ClickHouse (TTL: {} days)",
            self.retention_days
        );

        Ok(())
    }
}
