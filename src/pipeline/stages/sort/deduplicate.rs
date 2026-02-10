use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashSet;

pub struct DeduplicateStage;

#[async_trait]
impl PipelineStage for DeduplicateStage {
    fn name(&self) -> &str {
        "deduplicate"
    }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let mut seen = HashSet::new();
        let deduplicated: Vec<ScoredItem> = input.into_iter()
            .filter(|item| seen.insert(item.item_id))
            .collect();

        Ok(deduplicated)
    }
}
