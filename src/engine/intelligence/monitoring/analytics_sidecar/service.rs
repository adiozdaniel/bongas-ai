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
use tokio::sync::Mutex as TokioMutex;

use crate::engine::coordination::service::BongasEngine;
use crate::db::ResilientPool;

/// High-velocity event structure for ClickHouse.
#[derive(Debug, Clone, clickhouse::Row, serde::Serialize)]
pub struct UserEvent {
    pub user_id: i32,
    pub profile_id: Option<String>,
    pub request_id: String,
    pub item_id: i32,
    pub interaction_type: String,
    pub scenario_slug: Option<String>,
    pub device_type: Option<String>,
    pub watch_duration_seconds: i32,
    pub created_at: u64,
}

/// The sidecar that gives the binary "eyes" on its own performance.
pub struct AnalyticsSidecar {
    // We use Weak to avoid cyclic dependency with BongasEngine
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    clickhouse: ClickHouseClient,
    pool: Arc<ResilientPool>,
    check_interval: Duration,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    
    /// Internal high-velocity event buffer
    event_buffer: TokioMutex<Vec<UserEvent>>,
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
            event_buffer: TokioMutex::new(Vec::with_capacity(1000)),
        }
    }

    /// Record an event into the asynchronous buffer.
    pub async fn record_event(&self, event: UserEvent) {
        let mut buffer = self.event_buffer.lock().await;
        buffer.push(event);
        
        // Immediate flush if buffer is getting large
        if buffer.len() >= 1000 {
            drop(buffer);
            let _ = self.flush_events().await;
        }
    }

    /// Flush all buffered events to ClickHouse in a single batch.
    pub async fn flush_events(&self) -> Result<()> {
        let mut buffer = self.event_buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }

        let events_to_flush = std::mem::replace(&mut *buffer, Vec::with_capacity(1000));
        drop(buffer);

        let count = events_to_flush.len();
        
        // Execute batch insert
        let mut insert = self.clickhouse.insert::<UserEvent>("user_events").await?;
        for event in events_to_flush {
            insert.write(&event).await?;
        }
        insert.end().await?;

        debug!(count, "Successfully flushed batch to ClickHouse");
        Ok(())
    }

    /// Accessor for the ClickHouse client.
    pub fn clickhouse_client(&self) -> Option<ClickHouseClient> {
        Some(self.clickhouse.clone())
    }

    /// Set the engine reference (must be called after BongasEngine is created).
    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(engine);
    }

    /// Start the sidecar background loop.
    pub async fn start(self: Arc<Self>) {
        let mut analysis_ticker = interval(self.check_interval);
        let mut flush_ticker = interval(Duration::from_secs(10)); // Flush every 10s
        let mut shutdown_rx = self.shutdown_rx.resubscribe();
        
        info!("Analytics Sidecar started (Self-Awareness: ON)");

        loop {
            tokio::select! {
                _ = analysis_ticker.tick() => {
                    if let Err(e) = self.run_analysis_cycle().await {
                        error!(error = %e, "Analytics sidecar cycle failed");
                    }
                }
                _ = flush_ticker.tick() => {
                    if let Err(e) = self.flush_events().await {
                        error!(error = %e, "Failed to flush analytics events to ClickHouse");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Analytics Sidecar shutting down...");
                    let _ = self.flush_events().await; // Final flush
                    break;
                }
            }
        }
    }

    /// Run a single cycle of performance analysis.
    async fn run_analysis_cycle(&self) -> Result<()> {
        debug!("Running hourly self-performance analysis...");

        let engine_arc: Option<Arc<BongasEngine>> = {
            let guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|w: &Weak<BongasEngine>| w.upgrade())
        };

        if let Some(engine) = engine_arc {
            let scenario_slugs: Vec<String> = engine.list_scenarios().await;
            if scenario_slugs.is_empty() {
                warn!("No scenarios found in engine for analysis.");
                return Ok(());
            }

            let target_slug = if scenario_slugs.contains(&"home_feed".to_string()) {
                "home_feed".to_string()
            } else {
                scenario_slugs[0].clone()
            };

            // 1. Analyze Catalog Coverage
            let total_active_items = engine.execution.manager.item_feature_service.get_active_item_count().await?;
            let blindness = self.get_catalog_blindness(total_active_items).await?;
            if blindness > 0.6 { 
                let suggestion_id = self.suggest_discovery_boost(&target_slug, blindness).await?;
                if let Some(id) = suggestion_id {
                    // CLOSED-LOOP: Attempt autonomous promotion
                    let _ = engine.simulate_and_promote(id).await;
                }
            }

            // 2. Analyze Persona Churn
            let churned_users = self.get_user_churn().await?;
            for user_id in churned_users {
                let suggestion_id = self.suggest_retention_strategy(&target_slug, user_id).await?;
                if let Some(id) = suggestion_id {
                    // CLOSED-LOOP: Attempt autonomous promotion
                    let _ = engine.simulate_and_promote(id).await;
                }
            }
        }

        Ok(())
    }

    async fn get_catalog_blindness(&self, total_active_items: i64) -> Result<f64> {
        let query = format!(
            "SELECT (1 - (count(DISTINCT item_id) / {})) as blindness \
             FROM user_events \
             WHERE created_at > (toUnixTimestamp(now()) - 86400) \
               AND interaction_type = 'impression'",
            total_active_items.max(1)
        );

        let res: f64 = self.clickhouse.query(&query).fetch_one().await
            .context("Failed to fetch catalog blindness from ClickHouse")?;

        info!(blindness = format!("{:.2}%", res * 100.0), "Catalog coverage analysis complete");
        Ok(res)
    }

    async fn get_user_churn(&self) -> Result<Vec<i32>> {
        let query = r#"
            SELECT user_id
            FROM user_events
            WHERE created_at > (toUnixTimestamp(now()) - 604800)
            GROUP BY user_id
            HAVING (avgIf(watch_duration_seconds, created_at > (toUnixTimestamp(now()) - 86400)) < 
                   (avgIf(watch_duration_seconds, created_at <= (toUnixTimestamp(now()) - 86400)) * 0.8))
        "#;

        let results: Vec<i32> = self.clickhouse.query(query).fetch_all().await
            .context("Failed to fetch user churn from ClickHouse")?;

        Ok(results)
    }

    async fn suggest_retention_strategy(&self, scenario_slug: &str, user_id: i32) -> Result<Option<i32>> {
        let reasoning = format!("Engagement Churn Detected for user_id '{}'. Suggesting Retention strategy.", user_id);
        let condition = serde_json::json!({ "user_id": user_id });
        let s_slug = scenario_slug.to_string();

        let res: Option<i32> = self.pool.execute(move |pool| {
            let reason = reasoning.clone();
            let cond = condition.clone();
            let s = s_slug.clone();
            async move {
                sqlx::query_scalar(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    SELECT s.id, p.id, $2, $3, 0.85, 'pending'
                    FROM scenarios s, pipelines p
                    WHERE s.slug = $1 
                      AND (p.slug LIKE '%retention%' OR p.slug = 'retention_v1')
                    LIMIT 1
                    ON CONFLICT (scenario_id, suggested_pipeline_id, md5(suggested_condition::text)) WHERE status = 'pending' DO NOTHING
                    RETURNING id
                    "#
                )
                .bind(s).bind(cond).bind(reason)
                .fetch_optional(&pool).await
            }
        }).await?;

        Ok(res)
    }

    async fn suggest_discovery_boost(&self, scenario_slug: &str, blindness: f64) -> Result<Option<i32>> {
        let reasoning = format!("Catalog Blindness at {:.2}%. Suggesting Discovery-Heavy strategy.", blindness * 100.0);
        let s_slug = scenario_slug.to_string();

        let res: Option<i32> = self.pool.execute(move |pool| {
            let reason = reasoning.clone();
            let s = s_slug.clone();
            async move {
                sqlx::query_scalar(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    SELECT s.id, p.id, '{}'::jsonb, $2, 0.9, 'pending'
                    FROM scenarios s, pipelines p
                    WHERE s.slug = $1 
                      AND (p.slug LIKE '%discovery%' OR p.slug = 'discovery_v1')
                    LIMIT 1
                    ON CONFLICT (scenario_id, suggested_pipeline_id, md5(suggested_condition::text)) WHERE status = 'pending' DO NOTHING
                    RETURNING id
                    "#
                )
                .bind(s).bind(reason)
                .fetch_optional(&pool).await
            }
        }).await?;

        Ok(res)
    }
}
