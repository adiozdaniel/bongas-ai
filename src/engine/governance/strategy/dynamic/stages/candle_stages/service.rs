use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use crate::pipeline::ranking::candle_inference::service::CandleInferenceStage;

pub struct CandleDynamicStage;

#[async_trait]
impl PipelineStage for CandleDynamicStage {
    fn name(&self) -> &str { "candle_dynamic" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // Dynamic wrapper for Candle inference
        let inner = CandleInferenceStage;
        inner.execute(context, params, input).await
    }
}
