use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Deserialize)]
struct Params {
    /// Maximum boost for a brand new item with perfect affinity
    #[serde(default = "default_max_boost")]
    max_boost: f32,
    /// Time window in hours for freshness boost
    #[serde(default = "default_freshness_window")]
    freshness_window_hours: i32,
    /// The metadata key to check for affinity (e.g., "tribe", "language", "genre")
    #[serde(default = "default_affinity_key")]
    affinity_key: String,
}

fn default_max_boost() -> f32 { 5.0 }
fn default_freshness_window() -> i32 { 48 }
fn default_affinity_key() -> String { "genre".to_string() }

/// Identity-Aware Discovery: Boosts new content only if it matches user affinities.
pub struct AffinityFreshnessStage;

#[async_trait]
impl PipelineStage for AffinityFreshnessStage {
    fn name(&self) -> &str {
        "affinity_freshness"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(input),
        };

        // Fetch user preferences for affinity matching
        let user_prefs = context.item_feature_service.get_user_features(user_id).await?;
        let user_prefs = match user_prefs {
            Some(p) => p,
            None => return Ok(input),
        };

        // For this intelligent stage, we use genre_affinity as a proxy for identity/affinity
        let affinities: HashMap<String, f32> = user_prefs.genre_affinity
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let now = Utc::now();
        let window_secs = (params.freshness_window_hours * 3600) as f64;

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                // 1. Calculate Freshness Factor
                let published_at = item.metadata.get("published_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let freshness_factor = if let Some(dt) = published_at {
                    let age_secs = now.signed_duration_since(dt).num_seconds().max(0) as f64;
                    if age_secs < window_secs {
                        (1.0 - (age_secs / window_secs)) as f32
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // 2. Calculate Affinity Match
                let mut affinity_match = 0.0f32;
                if let Some(tags) = item.metadata.get(&params.affinity_key) {
                    if let Some(tag_list) = tags.as_array() {
                        for tag in tag_list {
                            if let Some(tag_str) = tag.as_str() {
                                if let Some(&score) = affinities.get(&tag_str.to_lowercase()) {
                                    affinity_match = affinity_match.max(score);
                                }
                            }
                        }
                    } else if let Some(tag_str) = tags.as_str() {
                        affinity_match = *affinities.get(&tag_str.to_lowercase()).unwrap_or(&0.0);
                    }
                }

                // 3. Intelligent Boost: Freshness * Affinity
                // New Gikuyu song for Luhya -> Fresh (1.0) * Affinity (0.0) = 0 boost
                // New Luhya comedy for Luhya -> Fresh (1.0) * Affinity (1.0) = Max boost
                if freshness_factor > 0.0 && affinity_match > 0.0 {
                    let boost = 1.0 + (params.max_boost - 1.0) * freshness_factor * affinity_match;
                    item.score *= boost;
                    item.metadata["discovery_boost"] = serde_json::json!(boost);
                    item.reasoning.push(format!(
                        "AffinityFreshness: +{:.2}x (freshness={:.2}, affinity={:.2})", 
                        boost, freshness_factor, affinity_match
                    ));
                }

                item
            })
            .collect();

        Ok(boosted)
    }
}
