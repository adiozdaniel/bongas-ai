//! AI assistant and rule suggestion logic.

use anyhow::{Result, Context};
use std::sync::Arc;
use tracing::{info, warn};
use crate::engine::coordination::service::{BongasEngine, RecommendationItem};
use crate::pipeline::ExecutionContext;

/// 🤖 AI: Strategic rule generation and optimization.
pub struct SuggestionsManager;

impl Default for SuggestionsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SuggestionsManager {
    pub fn new() -> Self {
        Self
    }
}

type RawSuggestionRow = (i32, String, String, serde_json::Value, Option<String>, Option<f64>, String, chrono::DateTime<chrono::Utc>);

impl BongasEngine {
    /// List all pending rule suggestions from the Analytics Sidecar.
    pub async fn list_suggestions(&self) -> Result<Vec<serde_json::Value>> {
        let rows: Vec<RawSuggestionRow> = self.governance.scenarios.scenario_factory.repo().pool().execute(|pool| async move {
            sqlx::query_as::<_, RawSuggestionRow>(
                r#"
                SELECT 
                    rs.id, s.slug as scenario_slug, p.slug as suggested_pipeline,
                    rs.suggested_condition, rs.reasoning, rs.confidence_score, rs.status, rs.created_at
                FROM bongas.rule_suggestions rs
                JOIN bongas.scenarios s ON rs.scenario_id = s.id
                JOIN bongas.pipelines p ON rs.suggested_pipeline_id = p.id
                WHERE rs.status = 'pending'
                ORDER BY rs.confidence_score DESC, rs.created_at DESC
                "#
            )
            .fetch_all(&pool)
            .await
        })
        .await
        .context("Failed to list rule suggestions")?;

        let json_list = rows.into_iter().map(|r| serde_json::json!({
            "id": r.0,
            "scenario_slug": r.1,
            "suggested_pipeline": r.2,
            "suggested_condition": r.3,
            "reasoning": r.4,
            "confidence_score": r.5,
            "status": r.6,
            "created_at": r.7,
        })).collect();

        Ok(json_list)
    }

    /// Approve a rule suggestion, promoting it to an active rule.
    pub async fn approve_suggestion(&self, suggestion_id: i32) -> Result<()> {
        info!(id = suggestion_id, "Approving rule suggestion...");

        self.governance.scenarios.scenario_factory.repo().pool().execute(move |pool| async move {
            let mut tx = pool.begin().await?;

            let suggestion: (i32, i32, serde_json::Value) = sqlx::query_as(
                "SELECT scenario_id, suggested_pipeline_id, suggested_condition FROM bongas.rule_suggestions WHERE id = $1"
            )
            .bind(suggestion_id)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO bongas.scenario_rules (scenario_id, pipeline_id, condition, priority, is_active, description)
                VALUES ($1, $2, $3, 150, true, 'AI Suggested & Autonomous Promoted')
                "#
            )
            .bind(suggestion.0)
            .bind(suggestion.1)
            .bind(&suggestion.2)
            .execute(&mut *tx)
            .await?;

            sqlx::query("UPDATE bongas.rule_suggestions SET status = 'approved', applied_at = NOW() WHERE id = $1")
                .bind(suggestion_id)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            Ok(())
        })
        .await?;

        self.reload_scenarios().await?;
        Ok(())
    }

    /// CLOSED-LOOP: Simulate and promote if safe (Confidence > 0.95)
    pub async fn simulate_and_promote(&self, suggestion_id: i32) -> Result<bool> {
        info!(id = suggestion_id, "🎼 Starting Autonomous Strategy Promotion cycle...");

        let impact = self.simulate_suggestion(suggestion_id).await?;
        
        // Extract comparison results
        let comparisons = impact.get("impact_comparison").and_then(|v| v.as_array())
            .context("Failed to parse simulation impact")?;

        let mut total_confidence = 0.0;
        let mut samples = 0;

        for comp in comparisons {
            let control_ids: Vec<i32> = comp.get("control_ids").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect())
                .unwrap_or_default();
            
            let suggested_ids: Vec<i32> = comp.get("suggested_ids").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect())
                .unwrap_or_default();

            let control_items: Vec<RecommendationItem> = control_ids.into_iter().map(|id| RecommendationItem {
                item_id: id, score: 1.0, metadata: serde_json::Value::Null, reasoning: vec![]
            }).collect();

            let suggested_items: Vec<RecommendationItem> = suggested_ids.into_iter().map(|id| RecommendationItem {
                item_id: id, score: 1.0, metadata: serde_json::Value::Null, reasoning: vec![]
            }).collect();

            let confidence = self.intelligence.simulator.calculate_drift_confidence(&control_items, &suggested_items);
            
            if !self.intelligence.simulator.validate_model_integrity(&suggested_items)? {
                warn!(id = suggestion_id, "Autonomous Promotion REJECTED: Model integrity check failed (Structural Instability)");
                return Ok(false);
            }

            total_confidence += confidence;
            samples += 1;
        }

        let average_confidence = total_confidence / samples.max(1) as f64;
        info!(id = suggestion_id, confidence = format!("{:.2}%", average_confidence * 100.0), "Simulation complete");

        if average_confidence > 0.95 {
            info!(id = suggestion_id, "🚀 High Confidence detected. Promoting strategy autonomously.");
            self.approve_suggestion(suggestion_id).await?;
            Ok(true)
        } else {
            warn!(id = suggestion_id, confidence = format!("{:.2}%", average_confidence * 100.0), "Autonomous Promotion SKIPPED: Drift too high for auto-approval");
            Ok(false)
        }
    }

    /// Simulate a rule suggestion before approval to see its impact.
    pub async fn simulate_suggestion(&self, suggestion_id: i32) -> Result<serde_json::Value> {
        info!(id = suggestion_id, "Simulating rule suggestion impact...");

        let (scenario_slug, suggested_p_id, condition): (String, i32, serde_json::Value) = self.governance.scenarios.scenario_factory.repo().pool().execute(move |pool| async move {
            sqlx::query_as(
                r#"
                SELECT s.slug, rs.suggested_pipeline_id, rs.suggested_condition 
                FROM bongas.rule_suggestions rs 
                JOIN bongas.scenarios s ON rs.scenario_id = s.id 
                WHERE rs.id = $1
                "#
            )
            .bind(suggestion_id)
            .fetch_one(&pool)
            .await
        }).await?;

        let p_def_json: serde_json::Value = self.governance.scenarios.scenario_factory.repo().pool().execute(move |pool| async move {
            sqlx::query_scalar("SELECT definition FROM bongas.pipelines WHERE id = $1")
                .bind(suggested_p_id)
                .fetch_one(&pool)
                .await
        }).await?;
        
        let p_def: crate::db::PipelineDefinition = serde_json::from_value(p_def_json)?;
        let suggested_pipeline = Arc::new(self.execution.pipeline_executor.link(&p_def)?);
        let control_pipeline = self.governance.scenarios.linked_scenarios.load().get(&scenario_slug).cloned();

        let sample_users: Vec<i32> = self.governance.scenarios.scenario_factory.repo().pool().execute(|pool| async move {
            sqlx::query_scalar::<_, i32>("SELECT DISTINCT user_id FROM bongas.user_interactions LIMIT 5")
                .fetch_all(&pool)
                .await
        }).await.unwrap_or_else(|_| vec![1, 2, 3]);

        let mut results = Vec::new();

        for uid in sample_users {
            let request_id = uuid::Uuid::new_v4().to_string();
            let mut context = ExecutionContext::new(
                Some(uid),
                self.execution.cache_manager.clone(),
                self.execution.model_loader.clone(),
                self.execution.item_feature_service.clone(),
                self.execution.feature_store.clone(),
                request_id,
            ).with_hot_registry(self.execution.manager.hot_registry.clone());

            if let Some(obj) = condition.as_object() {
                if let Some(pid) = obj.get("context.profile_id").and_then(|v| v.as_str()) {
                    context = context.with_profile_id(pid.to_string());
                }
                if let Some(mat) = obj.get("context.maturity_rating").and_then(|v| v.as_str()) {
                    context = context.with_maturity_rating(mat.to_string());
                }
                if let Some(dev) = obj.get("context.device_type").and_then(|v| v.as_str()) {
                    context = context.with_device_type(dev.to_string());
                }
            }

            let control_items: Vec<crate::engine::coordination::service::RecommendationItem> = if let Some(ref cp) = control_pipeline {
                self.execution.pipeline_executor.execute_linked(cp, &context).await?.into_iter().map(|item| crate::engine::coordination::service::RecommendationItem {
                    item_id: item.item_id,
                    score: item.score,
                    metadata: item.metadata,
                    reasoning: item.reasoning,
                }).collect()
            } else {
                Vec::new()
            };
            
            let suggested_items: Vec<crate::engine::coordination::service::RecommendationItem> = self.execution.pipeline_executor.execute_linked(&suggested_pipeline, &context).await?.into_iter().map(|item| crate::engine::coordination::service::RecommendationItem {
                item_id: item.item_id,
                score: item.score,
                metadata: item.metadata,
                reasoning: item.reasoning,
            }).collect();

            results.push(serde_json::json!({
                "user_id": uid,
                "control_ids": control_items.iter().take(5).map(|i| i.item_id).collect::<Vec<_>>(),
                "suggested_ids": suggested_items.iter().take(5).map(|i| i.item_id).collect::<Vec<_>>(),
            }));
        }

        Ok(serde_json::json!({
            "suggestion_id": suggestion_id,
            "scenario": scenario_slug,
            "sample_size": results.len(),
            "impact_comparison": results
        }))
    }

    /// Process a natural language query and convert it to a rule suggestion.
    pub async fn chatbot_process_query(&self, query: &str) -> Result<i32> {
        info!(query = %query, "AI Assistant processing natural language query...");

        let mut condition = serde_json::json!({});
        let mut reasoning = format!("AI Assistant translated: '{}'", query);

        if query.to_lowercase().contains("smart tv") || query.to_lowercase().contains("tv") {
            condition["context.device_type"] = serde_json::json!("tv");
        }
        
        let target_scenario = {
            let slugs: Vec<String> = self.list_scenarios().await;
            if slugs.contains(&"home_feed".to_string()) {
                "home_feed".to_string()
            } else if ! slugs.is_empty() {
                slugs[0].clone()
            } else {
                return Err(anyhow::anyhow!("No active scenarios found for chatbot processing"));
            }
        };

        let suggested_pipeline_slug: String = self.governance.scenarios.scenario_factory.repo().pool().execute(|pool| async move {
            let query_lower = query.to_lowercase();
            let target_slug = if query_lower.contains("diverse") || query_lower.contains("variety") {
                "discovery"
            } else if query_lower.contains("personal") || query_lower.contains("retention") {
                "retention"
            } else {
                "v1"
            };

            sqlx::query_scalar::<_, String>(
                "SELECT slug FROM bongas.pipelines WHERE slug LIKE $1 OR slug LIKE $2 LIMIT 1"
            )
            .bind(format!("%{}%", target_slug))
            .bind("%v1%")
            .fetch_one(&pool)
            .await
        }).await.context("Failed to resolve suggested pipeline for chatbot")?;

        if suggested_pipeline_slug.contains("discovery") {
            reasoning += " (Optimized for Catalog Coverage)";
        }

        let suggestion_id: i32 = self.governance.scenarios.scenario_factory.repo().pool().execute(move |pool| {
            let p_slug = suggested_pipeline_slug.clone();
            let cond = condition.clone();
            let reason = reasoning.clone();
            let s_slug = target_scenario.clone();
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
                        0.85,
                        'pending'
                    )
                    RETURNING id
                    "#
                )
                .bind(s_slug)
                .bind(p_slug)
                .bind(cond)
                .bind(reason)
                .fetch_one(&pool)
                .await
            }
        }).await?;

        Ok(suggestion_id)
    }

    /// Reject a rule suggestion.
    pub async fn reject_suggestion(&self, suggestion_id: i32) -> Result<()> {
        self.governance.scenarios.scenario_factory.repo().pool().execute(move |pool| async move {
            sqlx::query("UPDATE bongas.rule_suggestions SET status = 'rejected' WHERE id = $1")
                .bind(suggestion_id)
                .execute(&pool)
                .await
        })
        .await?;

        info!(id = suggestion_id, "Rule suggestion rejected");
        Ok(())
    }
}
