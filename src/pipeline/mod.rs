pub mod executor;
pub mod stages;
pub mod context;
pub mod registry;
pub mod validator;

use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::context::ExecutionContext;
use std::sync::Arc;
use std::time::Duration;
use crate::circuit_breaker::CircuitBreaker;

/// The type of data that a stage expects or produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StageDataKind {
    /// No input required (typical for the first stage)
    Empty,
    /// A list of raw item IDs (e.g., from a database fetch)
    ItemIds,
    /// A list of ScoredItems (items that have been ranked or filtered)
    ScoredItems,
}

/// Core trait for all pipeline stages
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Stage name (must match JSONB "type" field)
    fn name(&self) -> &str;

    /// The type of data this stage expects as input.
    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    /// The type of data this stage produces as output.
    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    /// Execute stage logic
    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>>;

    /// Returns true if this stage can be executed in parallel with other similar stages
    /// (e.g. multiple fetchers or multiple enrichers).
    fn can_parallelize(&self) -> bool {
        false
    }
}

/// Item with relevance score
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoredItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: JsonValue,
}

/// A stage that has been pre-linked with its implementation and circuit breaker.
/// This eliminates HashMap lookups during the request hot path.
pub struct BoundStage {
    pub implementation: Arc<dyn PipelineStage>,
    pub breaker: Option<Arc<CircuitBreaker>>,
    pub params: JsonValue,
    pub stage_type: String,
    pub timeout: Duration,
}

impl std::fmt::Debug for BoundStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundStage")
            .field("stage_type", &self.stage_type)
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// A node in the execution graph. Can be a single stage or a group of parallel stages.
#[derive(Debug)]
pub enum ExecutionNode {
    /// A single stage that must run in sequence (a sync barrier).
    Single(BoundStage),
    /// A group of stages that can run in parallel.
    Parallel(Vec<BoundStage>),
}

/// An executable pipeline where all stages have been pre-linked and grouped into execution nodes.
#[derive(Debug)]
pub struct ExecutablePipeline {
    pub nodes: Vec<ExecutionNode>,
    pub fallback_nodes: Option<Vec<ExecutionNode>>,
}

/// Specialized error for pipeline execution to aid in resilience routing.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Stage '{stage_type}' failed: {source}")]
    StageFailure {
        stage_type: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("Stage '{stage_type}' timed out after {timeout_ms}ms")]
    StageTimeout {
        stage_type: String,
        timeout_ms: u64,
    },
    #[error("Circuit breaker open for stage '{stage_type}'")]
    CircuitOpen {
        stage_type: String,
    },
    #[error("Pipeline execution timed out")]
    PipelineTimeout,
    #[error("Type mismatch in pipeline: stage '{stage_index}' ({stage_type}) outputs {output:?}, but stage '{next_stage_index}' ({next_stage_type}) expects {input:?}")]
    TypeMismatch {
        stage_index: usize,
        stage_type: String,
        output: StageDataKind,
        next_stage_index: usize,
        next_stage_type: String,
        input: StageDataKind,
    },
}
