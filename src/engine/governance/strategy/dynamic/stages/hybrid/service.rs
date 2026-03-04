use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    cf_weight: f32,
    content_weight: f32,
}

pub struct HybridStage;

#[async_trait]
impl PipelineStage for HybridStage {
    fn name(&self) -> &str { "hybrid" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse hybrid params")?;

        let mut output = input;
        for item in output.iter_mut() {
            // Simplified hybrid logic: assume metadata already contains model scores
            let cf_score = item.metadata.get("cf_score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let content_score = item.metadata.get("content_score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            
            item.score = (cf_score * params.cf_weight) + (content_score * params.content_weight);
        }

        Ok(output)
    }
}
