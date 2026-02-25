use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::stages::ml::ONNXInferenceStage;

pub struct ONNXDynamicStage;

#[async_trait]
impl PipelineStage for ONNXDynamicStage {
    fn name(&self) -> &str { "onnx_dynamic" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // Dynamic wrapper for ONNX inference
        let inner = ONNXInferenceStage;
        inner.execute(context, params, input).await
    }
}
