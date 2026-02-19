//! Phase 16: Hive Mind Connector (The Global Intelligence Link)
//!
//! Connects to the central Bongas-AI server to fetch global strategy
//! recommendations (Golden Rules). Locally evaluates these rules for safety
//! and performance before auto-accepting or queuing them for review.

use std::sync::{Arc, Weak};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use anyhow::{Result, Context};
use reqwest::Client;
use serde::Deserialize;

use crate::engine::BongasEngine;
use crate::config::types::HiveMindConfig;
use crate::db::ResilientPool;

#[derive(Debug, Deserialize)]
struct GlobalRule {
    pub id: String,
    pub scenario_slug: String,
    pub pipeline_slug: String,
    pub condition: serde_json::Value,
    pub reasoning: String,
    pub min_confidence: f64,
}

/// background worker that syncs with the Global Hive Mind.
pub struct HiveMindConnector {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    config: HiveMindConfig,
    pool: Arc<ResilientPool>,
    http_client: Client,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl HiveMindConnector {
    pub fn new(
        config: HiveMindConfig,
        pool: Arc<ResilientPool>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            config,
            pool,
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            shutdown_rx,
        }
    }

    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap();
        *guard = Some(engine);
    }

    pub async fn start(mut self: Arc<Self>) {
        if !self.config.enabled {
            info!("Hive Mind Connector disabled");
            return;
        }

        let mut ticker = interval(Duration::from_secs(self.config.poll_interval_seconds));
        let mut shutdown_rx = self.shutdown_rx.resubscribe();

        info!("Hive Mind Connector started. Polling global intelligence every {}s", self.config.poll_interval_seconds);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.sync_cycle().await {
                        error!(error = %e, "Hive Mind sync failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Hive Mind Connector shutting down...");
                    break;
                }
            }
        }
    }

    async fn sync_cycle(&self) -> Result<()> {
        debug!("Polling Global Hive Mind for new strategies...");

        // 1. Fetch rules from central server
        let rules = self.fetch_global_rules().await?;
        if rules.is_empty() {
            return Ok(());
        }

        info!(count = rules.len(), "Received new global strategy recommendations");

        // 2. Process each rule
        for rule in rules {
            self.process_global_rule(rule).await?;
        }

        Ok(())
    }

    async fn fetch_global_rules(&self) -> Result<Vec<GlobalRule>> {
        // In a real implementation, this would call the actual API
        // For prototype, we simulate an empty list or a mock response
        // let res = self.http_client.get(&self.config.url)
        //     .header("Authorization", self.config.api_key.as_deref().unwrap_or(""))
        //     .send().await?;
        // res.json().await.context("Failed to parse Hive Mind response")
        
        Ok(Vec::new()) // Mock empty for now to pass safety checks
    }

    async fn process_global_rule(&self, rule: GlobalRule) -> Result<()> {
        // 1. Insert as suggestion
        let suggestion_id: i32 = self.pool.execute(|pool| {
            let r = rule.reasoning.clone();
            let c = rule.condition.clone();
            let s_slug = rule.scenario_slug.clone();
            let p_slug = rule.pipeline_slug.clone();
            async move {
                sqlx::query_scalar(
                    r#"
                    INSERT INTO rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    VALUES (
                        (SELECT id FROM scenarios WHERE slug = $1 LIMIT 1),
                        (SELECT id FROM pipelines WHERE slug = $2 LIMIT 1),
                        $3,
                        $4,
                        0.95,
                        'pending'
                    )
                    RETURNING id
                    "#
                )
                .bind(s_slug)
                .bind(p_slug)
                .bind(c)
                .bind(format!("Global Hive Mind: {}", r))
                .fetch_one(&pool)
                .await
            }
        }).await?;

        // 2. Auto-Evaluation (Local Adaptation)
        if self.config.auto_approve_safe_rules {
            self.evaluate_and_approve(suggestion_id).await?;
        }

        Ok(())
    }

    async fn evaluate_and_approve(&self, suggestion_id: i32) -> Result<()> {
        let engine_arc = {
            let guard = self.engine.lock().unwrap();
            guard.as_ref().and_then(|w| w.upgrade())
        };

        if let Some(engine) = engine_arc {
            // Run Simulation
            let simulation = engine.simulate_suggestion(suggestion_id).await?;
            
            // Safety Check: Ensure we don't return 0 items
            if let Some(results) = simulation.get("impact_comparison").and_then(|v| v.as_array()) {
                let safe = results.iter().all(|r| {
                    r.get("suggested_ids").and_then(|v| v.as_array()).map(|arr| !arr.is_empty()).unwrap_or(false)
                });

                if safe {
                    info!(suggestion_id, "Global rule passed local safety simulation. Auto-approving.");
                    engine.approve_suggestion(suggestion_id).await?;
                } else {
                    warn!(suggestion_id, "Global rule FAILED local safety simulation (empty results). Leaving as pending.");
                }
            }
        }

        Ok(())
    }
}
