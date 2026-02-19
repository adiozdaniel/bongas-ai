use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use chrono::Utc;

#[derive(Deserialize)]
struct Params {
    boost_factor: f32,
    decay_hours: i64,
}

pub struct BoostByRecencyStage;

#[async_trait]
impl PipelineStage for BoostByRecencyStage {
    fn name(&self) -> &str {
        "boost_by_recency"
    }

    fn is_fusable(&self) -> bool { true }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let boosted: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            if let Some(last_watched_str) = item.metadata.get("last_watched_at").and_then(|v| v.as_str()) {
                if let Ok(last_watched) = chrono::DateTime::parse_from_rfc3339(last_watched_str) {
                    let hours_ago = (Utc::now() - last_watched.with_timezone(&Utc)).num_hours();
                    let decay = (-(hours_ago as f32) / params.decay_hours as f32).exp();
                    item.score *= 1.0 + (params.boost_factor - 1.0) * decay;
                }
            }
            item
        }).collect();

        Ok(boosted)
    }
}
