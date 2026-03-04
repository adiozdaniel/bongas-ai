use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use crate::pipeline::ranking::diversify_genres::service::DiversifyGenresStage;

#[derive(Deserialize)]
struct Params {
    strategy: String,
}

pub struct DynamicDiversifiersStage;

#[async_trait]
impl PipelineStage for DynamicDiversifiersStage {
    fn name(&self) -> &str { "dynamic_diversifiers" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let stage_config: Params = serde_json::from_value(params.clone())
            .context("Failed to parse dynamic_diversifiers params")?;

        match stage_config.strategy.as_str() {
            "genre" => {
                let inner = DiversifyGenresStage;
                inner.execute(context, params, input).await
            }
            _ => {
                // Fallback to no-op for unknown strategies
                Ok(input)
            }
        }
    }
}
