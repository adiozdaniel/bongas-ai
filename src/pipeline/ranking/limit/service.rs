use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    count: usize,
}

pub struct LimitStage;

#[async_trait]
impl PipelineStage for LimitStage {
    fn name(&self) -> &str {
        "limit"
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        Ok(input.into_iter().take(params.count).collect())
    }
}
