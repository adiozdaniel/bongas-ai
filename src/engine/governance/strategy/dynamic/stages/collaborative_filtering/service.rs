use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use crate::pipeline::ranking::ml_inference_bert4rec::service::MLInferenceBERT4RecStage;

pub struct CollaborativeFilteringStage;

#[async_trait]
impl PipelineStage for CollaborativeFilteringStage {
    fn name(&self) -> &str { "collaborative_filtering" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let inner = MLInferenceBERT4RecStage;
        inner.execute(context, params, input).await
    }
}
