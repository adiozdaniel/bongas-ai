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

        // Use the engine reference to list scenarios (Phase 16: Use Weak Engine)
        let engine_arc = {
            let guard = self.engine.lock().unwrap();
            guard.as_ref().and_then(|w| w.upgrade())
        };

        if let Some(engine) = engine_arc {
            let scenario_slugs = engine.list_scenarios().await;
            if scenario_slugs.is_empty() {
                warn!("No scenarios found in engine for analysis.");
                return Ok(());
            }

            // Target scenarios based on popularity or just take the first few
            // For now, we prioritize 'home_feed' but fall back to others
            let target_slug = if scenario_slugs.contains(&"home_feed".to_string()) {
                "home_feed".to_string()
            } else {
                scenario_slugs[0].clone()
            };

            // 1. Analyze Catalog Coverage (Blindness)
            // Fetch total active items from Postgres first
            let total_active_items = engine.item_feature_service.get_active_item_count().await?;
            let blindness = self.get_catalog_blindness(total_active_items).await?;
            if blindness > 0.6 { 
                self.suggest_discovery_boost(&target_slug, blindness).await?;
            }

            // 2. Analyze Persona Churn (Drop in engagement)
            let churned_users = self.get_user_churn().await?;
            for user_id in churned_users {
                self.suggest_retention_strategy(&target_slug, user_id).await?;
            }
        } else {
            warn!("Analytics sidecar lost engine reference");
        }

        Ok(())
    }

    /// Calculate the percentage of items with 0 impressions in the last 24h.
    async fn get_catalog_blindness(&self, total_active_items: i64) -> Result<f64> {
        let query = format!(
            "SELECT (1 - (count(DISTINCT item_id) / {})) as blindness \
             FROM user_interactions \
             WHERE created_at > (toUnixTimestamp(now()) - 86400) \
               AND interaction_type = 'impression'",
            total_active_items.max(1)
        );

        let res: f64 = self.clickhouse.query(&query).fetch_one().await
            .context("Failed to fetch catalog blindness from ClickHouse")?;

        info!(blindness = format!("{:.2}%", res * 100.0), "Catalog coverage analysis complete");
        Ok(res)
    }

    /// Identify users with >20% drop in session duration compared to last week's baseline.
    async fn get_user_churn(&self) -> Result<Vec<i32>> {
        let query = r#"
            SELECT user_id
            FROM user_interactions
            WHERE created_at > (toUnixTimestamp(now()) - 604800)
            GROUP BY user_id
            HAVING (avgIf(watch_duration_seconds, created_at > (toUnixTimestamp(now()) - 86400)) < 
                   (avgIf(watch_duration_seconds, created_at <= (toUnixTimestamp(now()) - 86400)) * 0.8))
        "#;

        let results: Vec<i32> = self.clickhouse.query(query).fetch_all().await
            .context("Failed to fetch user churn from ClickHouse")?;

        if !results.is_empty() {
            info!(count = results.len(), "Engagement drop detected for multiple users");
        }
        Ok(results)
    }

    /// Insert a suggestion to switch to a retention-heavy pipeline for a specific user.
    async fn suggest_retention_strategy(&self, scenario_slug: &str, user_id: i32) -> Result<()> {
        let reasoning = format!(
            "Engagement Churn Detected for user_id '{}' in scenario '{}'. Duration dropped by >20%. Suggesting Personalized Retention strategy.",
            user_id, scenario_slug
        );

        let condition = serde_json::json!({
            "user_id": user_id
        });

        let s_slug = scenario_slug.to_string();

        let rows_affected = self.pool.execute(move |pool| {
            let reason = reasoning.clone();
            let cond = condition.clone();
            let s = s_slug.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    SELECT s.id, p.id, $2, $3, 0.85, 'pending'
                    FROM scenarios s, pipelines p
                    WHERE s.slug = $1 
                      AND (p.slug LIKE '%retention%' OR p.slug LIKE '%personalized%' OR p.slug = 'retention_v1')
                    ORDER BY p.id ASC
                    LIMIT 1
                    ON CONFLICT (scenario_id, suggested_pipeline_id, md5(suggested_condition::text)) WHERE status = 'pending' DO NOTHING
                    "#
                )
                .bind(s)
                .bind(cond)
                .bind(reason)
                .execute(&pool)
                .await
            }
        }).await?.rows_affected();

        if rows_affected == 0 {
            warn!(user_id = %user_id, scenario = %scenario_slug, "Retention strategy suggestion SKIPPED (No suitable pipeline found or duplicate exists)");
        } else {
            info!(user_id = %user_id, scenario = %scenario_slug, "Retention gap detected: Strategy suggestion pushed");
        }
        Ok(())
    }

    /// Insert a suggestion to switch to a discovery-heavy pipeline.
    async fn suggest_discovery_boost(&self, scenario_slug: &str, blindness: f64) -> Result<()> {
        let reasoning = format!(
            "Catalog Blindness is at {:.2}% in scenario '{}'. Discovery is currently suboptimal. Suggesting switch to Discovery-Heavy strategy.",
            blindness * 100.0, scenario_slug
        );

        let s_slug = scenario_slug.to_string();

        let rows_affected = self.pool.execute(move |pool| {
            let reason = reasoning.clone();
            let s = s_slug.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    SELECT s.id, p.id, '{}'::jsonb, $2, 0.9, 'pending'
                    FROM scenarios s, pipelines p
                    WHERE s.slug = $1 
                      AND (p.slug LIKE '%discovery%' OR p.slug LIKE '%coverage%' OR p.slug = 'discovery_v1')
                    ORDER BY p.id ASC
                    LIMIT 1
                    ON CONFLICT (scenario_id, suggested_pipeline_id, md5(suggested_condition::text)) WHERE status = 'pending' DO NOTHING
                    "#
                )
                .bind(s)
                .bind(reason)
                .execute(&pool)
                .await
            }
        }).await?.rows_affected();

        if rows_affected == 0 {
            warn!(scenario = %scenario_slug, "Discovery strategy suggestion SKIPPED (No suitable pipeline found or duplicate exists)");
        } else {
            info!(scenario = %scenario_slug, "Performance Gap Detected: Discovery suggestion pushed to admin queue");
        }
        Ok(())
    }
}
