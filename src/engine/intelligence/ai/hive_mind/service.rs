//! Phase 16: Hive Mind Connector (The Global Intelligence Link)
//!
//! Connects to the central Bongas-AI server to fetch global strategy
//! recommendations (Golden Rules). Locally evaluates these rules for safety
//! and performance before auto-accepting or queuing them for review.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Weak};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use anyhow::Result;
use reqwest::Client;

use crate::engine::coordination::service::BongasEngine;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseClassification {
    pub theme: String,
    pub kind: PulseKind,
    pub semantic_vector: Vec<f32>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminQuery {
    pub text: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminResponse {
    pub text: String,
    pub action: Option<serde_json::Value>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PulseKind {
    PhysicalEvent,
    Discourse,
}

/// background worker that syncs with the Global Hive Mind.
pub struct HiveMindConnector {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    config: HiveMindConfig,
    pool: Option<Arc<ResilientPool>>,
    http_client: Client,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl HiveMindConnector {
    pub fn new(
        config: HiveMindConfig,
        pool: Option<Arc<ResilientPool>>,
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
        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(engine);
    }

    /// Classify a regional headline into a theme and semantic vector.
    pub async fn classify_pulse(&self, headline: &str, location: &str) -> Result<PulseClassification> {
        debug!(headline, location, "Classifying regional pulse via Hive Mind");

        // Mock implementation: In production, this would call an LLM for classification and embeddings.
        let (theme, kind) = if headline.to_lowercase().contains("flood") || headline.to_lowercase().contains("storm") || headline.to_lowercase().contains("earthquake") {
            ("Natural Disaster", PulseKind::PhysicalEvent)
        } else if headline.to_lowercase().contains("festival") || headline.to_lowercase().contains("concert") {
            ("Cultural Event", PulseKind::PhysicalEvent)
        } else {
            ("General Discourse", PulseKind::Discourse)
        };

        // Generate a deterministic 128-dim mock vector based on the theme
        let mut semantic_vector = vec![0.0; 128];
        for (i, byte) in theme.as_bytes().iter().enumerate().take(128) {
            semantic_vector[i] = (*byte as f32) / 255.0;
        }

        Ok(PulseClassification {
            theme: theme.to_string(),
            kind,
            semantic_vector,
            confidence: 0.92,
        })
    }

    /// Generate a human-readable reason for a recommendation based on profile and item features.
    pub async fn generate_reasoning(
        &self,
        profile_id: &str,
        item_id: i32,
        genre_affinity: &serde_json::Value,
        item_tags: &serde_json::Value,
    ) -> Result<String> {
        debug!(profile_id, item_id, "Generating reasoning via Hive Mind");

        // Mock implementation: In production, this would call an LLM with the feature set.
        // We simulate this by matching tags to affinities.
        
        let affinities = genre_affinity.as_object();
        let tags = item_tags.as_array();

        if let (Some(aff), Some(ts)) = (affinities, tags) {
            for tag in ts {
                if let Some(tag_str) = tag.as_str() {
                    if aff.contains_key(tag_str) {
                        return Ok(format!("Because you enjoy {} content", tag_str));
                    }
                }
            }
        }

        Ok("Recommended based on your viewing patterns".to_string())
    }

    /// The Symphony Conductor (Admin ReAct Interface)
    /// 
    /// Implements behavioral rails:
    /// - Context Shielding (Point 1)
    /// - Conversational Fillers (Swahili/Sheng) (Point 2)
    /// - Forensic Reconciliation (uko sure wewe? / ni sawa basi) (Point 9)
    /// - Latency Masking (kiasi tu) (Point 10)
    /// - Instant Forensic Interpretations (Point 11)
    pub async fn process_admin_query(&self, query: AdminQuery) -> Result<AdminResponse> {
        let text = query.text.to_lowercase();
        
        // 1. Instant Forensic Interpretations (Pillar 3, Point 11)
        if text.contains("rated as") || text.contains("rating ya") {
            // Mock: Pull from ClickHouse in production
            return Ok(AdminResponse {
                text: "18+".to_string(), // Instant snapshot
                action: None,
                confidence: 1.0,
            });
        }

        // 2. Latency Masking (Pillar 3, Point 10)
        if text.contains("check") || text.contains("confirm") {
            // Simulate a slow check
            return Ok(AdminResponse {
                text: "kiasi tu... acha ni confirm kwanza".to_string(),
                action: Some(serde_json::json!({"status": "processing"})),
                confidence: 0.95,
            });
        }

        // 3. Forensic Reconciliation & Skepticism (Pillar 3, Point 9)
        if (text.contains("it is ge") || text.contains("ni ge"))
            && query.context.get("forensic_flag").and_then(|v| v.as_str()) == Some("17+")
        {
            return Ok(AdminResponse {
                text: "uko sure wewe? forensics inasema hii ni 17+".to_string(),
                action: Some(serde_json::json!({"requires_confirmation": true})),
                confidence: 0.99,
            });
        }

        if (text.contains("yes") || text.contains("ndio") || text.contains("i am sure"))
            && query.context.get("pending_override").and_then(|v| v.as_bool()) == Some(true)
        {
            return Ok(AdminResponse {
                text: "ni sawa basi. nime update metadata.".to_string(),
                action: Some(serde_json::json!({"operation": "manual_override", "status": "committed"})),
                confidence: 1.0,
            });
        }

        // 4. Context Shielding (Pillar 3, Point 1)
        if text.contains("weather") || text.contains("sports") || text.contains("football") {
            return Ok(AdminResponse {
                text: "Sawa, lakini wacha tu focus kwenye system orchestration na analytics kwa sasa.".to_string(),
                action: None,
                confidence: 1.0,
            });
        }

        // 5. Forbidden Actions (Pillar 3, Point 4)
        if text.contains("delete") || text.contains("futa") {
            return Ok(AdminResponse {
                text: "I can only hide it from appearing to users, but I am not allowed to permanently delete records.".to_string(),
                action: Some(serde_json::json!({"suggested_action": "soft_delete_or_hide"})),
                confidence: 1.0,
            });
        }

        // 6. Technical Translation (Pillar 3, Point 5)
        if text.contains("doubling weight") || text.contains("boost") || text.contains("similarity score") {
            return Ok(AdminResponse {
                text: "This will increase the availability of this content in the feeds.".to_string(),
                action: Some(serde_json::json!({"math_op": "boost_weight"})),
                confidence: 0.98,
            });
        }

        // Default response with fillers (Point 2)
        Ok(AdminResponse {
            text: "poa, nimekupata. ungetaka nisaidie na nini kulingana na engine?".to_string(),
            action: None,
            confidence: 0.8,
        })
    }

    pub async fn start(self: Arc<Self>) {
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
            // WIRE UP: Use min_confidence for filtering
            if rule.min_confidence > 0.99 {
                warn!(rule_id = %rule.id, confidence = rule.min_confidence, "Extremely high confidence global rule detected. Preparing for auto-evaluation.");
            }
            self.process_global_rule(rule).await?;
        }

        Ok(())
    }

    async fn fetch_global_rules(&self) -> Result<Vec<GlobalRule>> {
        // WIRE UP: Actually use the http_client
        if self.config.url.is_empty() {
            return Ok(Vec::new());
        }

        let mut request = self.http_client.get(&self.config.url);
        if let Some(ref key) = self.config.api_key {
            request = request.header("Authorization", key);
        }

        match request.send().await {
            Ok(res) => {
                if res.status().is_success() {
                    Ok(res.json().await.unwrap_or_default())
                } else {
                    warn!(status = %res.status(), "Hive Mind API returned non-success status");
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to Global Hive Mind");
                Ok(Vec::new())
            }
        }
    }

    async fn process_global_rule(&self, rule: GlobalRule) -> Result<()> {
        let pool = match &self.pool {
            Some(p) => p,
            None => {
                warn!("Cannot process global rule: Database pool not available");
                return Ok(());
            }
        };

        // 1. Insert as suggestion
        let suggestion_id: i32 = pool.execute(|pool| {
            let r = rule.reasoning.clone();
            let c = rule.condition.clone();
            let s_slug = rule.scenario_slug.clone();
            let p_slug = rule.pipeline_slug.clone();
            let conf = rule.min_confidence;
            async move {
                sqlx::query_scalar(
                    r#"
                    INSERT INTO bongas.rule_suggestions (
                        scenario_id, suggested_pipeline_id, suggested_condition, 
                        reasoning, confidence_score, status
                    )
                    VALUES (
                        (SELECT id FROM bongas.scenarios WHERE slug = $1 LIMIT 1),
                        (SELECT id FROM bongas.pipelines WHERE slug = $2 LIMIT 1),
                        $3,
                        $4,
                        $5,
                        'pending'
                    )
                    RETURNING id
                    "#
                )
                .bind(s_slug)
                .bind(p_slug)
                .bind(c)
                .bind(format!("Global Hive Mind: {}", r))
                .bind(conf)
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
        let engine_arc: Option<Arc<BongasEngine>> = {
            let guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|w: &Weak<BongasEngine>| w.upgrade())
        };

        if let Some(engine) = engine_arc {
            // Run Simulation
            let simulation: serde_json::Value = engine.simulate_suggestion(suggestion_id).await?;
            
            // Safety Check: Ensure we don't return 0 items
            if let Some(results) = simulation.get("impact_comparison").and_then(|v| v.as_array()) {
                let safe = results.iter().all(|r: &serde_json::Value| {
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
