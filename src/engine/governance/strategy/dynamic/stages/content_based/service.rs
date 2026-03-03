use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use crate::pipeline::stages::ml::MLInferenceSimilarityStage;

pub struct ContentBasedStage;

#[async_trait]
impl PipelineStage for ContentBasedStage {
    fn name(&self) -> &str { "content_based" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let inner = MLInferenceSimilarityStage;
        inner.execute(context, params, input).await
    }
}
