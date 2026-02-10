pub mod executor;
pub mod stages;
pub mod context;
pub mod registry;

use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::context::ExecutionContext;

/// Core trait for all pipeline stages
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Stage name (must match JSONB "type" field)
    fn name(&self) -> &str;

    /// Execute stage logic
    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>>;
}

/// Item with relevance score
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoredItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: JsonValue,
}
