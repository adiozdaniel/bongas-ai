//! Phase 16: Analytics Sidecar (The Self-Aware Observer)
//!
//! Monitors ClickHouse metrics to identify gaps in diversity and coverage.
//! Automatically generates strategic rule suggestions when performance
//! thresholds are breached.

use std::sync::{Arc, Weak};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use anyhow::{Result, Context};
use clickhouse::Client as ClickHouseClient;

use crate::engine::BongasEngine;
use crate::db::ResilientPool;

/// The sidecar that gives the binary "eyes" on its own performance.
pub struct AnalyticsSidecar {
    // We use Weak to avoid cyclic dependency with BongasEngine
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    clickhouse: ClickHouseClient,
    pool: Arc<ResilientPool>,
    check_interval: Duration,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl AnalyticsSidecar {
    pub fn new(
        clickhouse: ClickHouseClient,
        pool: Arc<ResilientPool>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            clickhouse,
            pool,
            check_interval: Duration::from_secs(3600), // Run hourly
            shutdown_rx,
        }
    }

    /// Set the engine reference (must be called after BongasEngine is created).
    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap();
        *guard = Some(engine);
    }

    /// Start the sidecar background loop.
    pub async fn start(self: Arc<Self>) {
        let mut ticker = interval(self.check_interval);
        let mut shutdown_rx = self.shutdown_rx.resubscribe();
        
        info!("Analytics Sidecar started (Self-Awareness: ON)");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_analysis_cycle().await {
                        error!(error = %e, "Analytics sidecar cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Analytics Sidecar shutting down...");
                    break;
                }
            }
        }
    }

    /// Run a single cycle of performance analysis.
    async fn run_analysis_cycle(&self) -> Result<()> {
        debug!("Running hourly self-performance analysis...");

        // 1. Analyze Catalog Coverage (Blindness)
        let blindness = self.get_catalog_blindness().await?;
        if blindness > 0.6 { // If 60% of catalog is never seen
            self.suggest_discovery_boost(blindness).await?;
        }

        Ok(())
    }

    /// Calculate the percentage of items with 0 impressions in the last 24h.
    async fn get_catalog_blindness(&self) -> Result<f64> {
        let query = r#"
            SELECT
                (1 - (count(DISTINCT item_id) / GREATEST((SELECT count() FROM item_features WHERE is_active = true), 1))) as blindness
            FROM user_interactions
            WHERE created_at > (toUnixTimestamp(now()) - 86400)
              AND interaction_type = 'impression'
        "#;

        let res: f64 = self.clickhouse.query(query).fetch_one().await
            .context("Failed to fetch catalog blindness from ClickHouse")?;

        info!(blindness = format!("{:.2}%", res * 100.0), "Catalog coverage analysis complete");
        Ok(res)
    }

    /// Insert a suggestion to switch to a discovery-heavy pipeline.
    async fn suggest_discovery_boost(&self, blindness: f64) -> Result<()> {
        let reasoning = format!(
            "Catalog Blindness is at {:.2}%. Discovery is currently suboptimal. Suggesting switch to Discovery-Heavy strategy.",
            blindness * 100.0
        );

        self.pool.execute(|pool| {
            let reason = reasoning.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    VALUES (
                        (SELECT id FROM scenarios WHERE slug = 'home_feed' LIMIT 1),
                        (SELECT id FROM pipelines WHERE slug = 'discovery_v1' LIMIT 1),
                        '{}'::jsonb,
                        $1,
                        0.9,
                        'pending'
                    )
                    ON CONFLICT DO NOTHING
                    "#
                )
                .bind(reason)
                .execute(&pool)
                .await
            }
        }).await?;

        warn!("Performance Gap Detected: Discovery suggestion pushed to admin queue");
        Ok(())
    }
}
