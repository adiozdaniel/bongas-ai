use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

pub struct ONNXDynamicStage;

#[async_trait]
impl PipelineStage for ONNXDynamicStage {
    fn name(&self) -> &str { "onnx_dynamic" }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // TODO: Implement dynamic ONNX inference logic
        Ok(input)
    }
}
