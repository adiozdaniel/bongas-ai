use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    descending: bool,
}

pub struct SortByScoreStage;

#[async_trait]
impl PipelineStage for SortByScoreStage {
    fn name(&self) -> &str {
        "sort_by_score"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let mut sorted = input;
        if params.descending {
            sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            sorted.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
        }

        Ok(sorted)
    }
}
