use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

pub struct FetchSearchResultsStage;

#[async_trait]
impl PipelineStage for FetchSearchResultsStage {
    fn name(&self) -> &str {
        "fetch_search_results"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        _context: &ExecutionContext,
        _params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        Err(anyhow::anyhow!("fetch_search_results (Meilisearch) is deprecated. Use fetch_embedded_search instead."))
    }
}
