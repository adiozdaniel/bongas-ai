use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Weight for base score (0.0-1.0)
    #[serde(default = "default_score_weight")]
    score_weight: f32,
    /// Weight for recency (0.0-1.0)
    #[serde(default = "default_recency_weight")]
    recency_weight: f32,
    /// Weight for personalization (0.0-1.0)
    #[serde(default = "default_personalization_weight")]
    personalization_weight: f32,
    /// Weight for engagement metrics (0.0-1.0)
    #[serde(default = "default_engagement_weight")]
    engagement_weight: f32,
    /// Descending order (highest first)
    #[serde(default = "default_true")]
    descending: bool,
    /// Boost items from specific sources
    #[serde(default)]
    boost_sources: Vec<String>,
    /// Amount to boost specified sources
    #[serde(default = "default_source_boost")]
    source_boost_amount: f32,
}

fn default_score_weight() -> f32 {
    0.4
}

fn default_recency_weight() -> f32 {
    0.2
}

fn default_personalization_weight() -> f32 {
    0.25
}

fn default_engagement_weight() -> f32 {
    0.15
}

fn default_true() -> bool {
    true
}

fn default_source_boost() -> f32 {
    0.1
}

pub struct SortByRelevanceStage;

#[async_trait]
impl PipelineStage for SortByRelevanceStage {
    fn name(&self) -> &str {
        "sort_by_relevance"
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let mut items: Vec<(ScoredItem, f32)> = input
            .into_iter()
            .map(|item| {
                // Extract metadata values for relevance calculation
                let recency_score = item.metadata.get("recency_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;

                let personalization_score = item.metadata.get("personalization_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;

                let engagement_score = item.metadata.get("engagement_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;

                // Calculate weighted relevance score
                let mut relevance =
                    item.score * params.score_weight +
                    recency_score * params.recency_weight +
                    personalization_score * params.personalization_weight +
                    engagement_score * params.engagement_weight;

                // Apply source boost if applicable
                if !params.boost_sources.is_empty() {
                    if let Some(source) = item.metadata.get("source").and_then(|v| v.as_str()) {
                        if params.boost_sources.iter().any(|s| s == source) {
                            relevance += params.source_boost_amount;
                        }
                    }
                }

                (item, relevance)
            })
            .collect();

        // Sort by calculated relevance
        if params.descending {
            items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Return items with updated scores
        Ok(items
            .into_iter()
            .map(|(mut item, relevance)| {
                item.score = relevance;
                item
            })
            .collect())
    }
}
