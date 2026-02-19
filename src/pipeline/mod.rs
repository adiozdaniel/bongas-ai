pub mod executor;
pub mod stages;
pub mod context;
pub mod registry;
pub mod validator;
pub mod optimizer;

use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::context::ExecutionContext;
use std::sync::Arc;
use std::time::Duration;
use crate::circuit_breaker::CircuitBreaker;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

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

    /// Returns true if this stage can be executed in parallel with other similar stages.
    fn can_parallelize(&self) -> bool {
        false
    }
}

/// Item with relevance score and dual-mode metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoredItem {
    pub item_id: i32,
    pub score: f32,
    pub metadata: JsonValue,
    #[serde(default)]
    pub reasoning: Vec<String>,
    pub fast_metadata: Option<Vec<u8>>,
}

impl ScoredItem {
    pub fn new(item_id: i32, score: f32, metadata: JsonValue) -> Self {
        Self {
            item_id,
            score,
            metadata,
            reasoning: Vec::new(),
            fast_metadata: None,
        }
    }
}

/// Zero-copy metadata schema for the hot path.
#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
#[archive(check_bytes)]
pub struct CompactMetadata {
    pub features: Vec<f32>,
    pub flags: u64,
    pub category_id: i32,
}

/// A stage that has been pre-linked with its implementation and circuit breaker.
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

/// Strategy for merging scores from multiple branches or parallel stages.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Sum,
    Max,
    Min,
    Average,
    First,
}

/// A node in the execution graph.
#[derive(Debug)]
pub enum ExecutionNode {
    Single(BoundStage),
    Parallel {
        stages: Vec<BoundStage>,
        merge_strategy: MergeStrategy,
    },
    /// A group of stages fused into a single pass (JIT-lite).
    Fused(Vec<BoundStage>),
    /// A conditional branch point.
    Branch {
        condition: BranchCondition,
        if_true: Vec<ExecutionNode>,
        if_false: Vec<ExecutionNode>,
    },
    /// A weighted ensemble of multiple source branches.
    Ensemble {
        sources: Vec<EnsembleSource>,
        merge_strategy: MergeStrategy,
    },
    /// A slot-based interleaver for discovery and retention.
    Interleave {
        pattern: Vec<String>,
        sources: std::collections::HashMap<String, Vec<ExecutionNode>>,
    },
}

/// Condition for pipeline branching.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BranchCondition {
    pub key: String,
    pub operator: String,
    pub value: serde_json::Value,
}

/// A weighted source for an ensemble node.
#[derive(Debug)]
pub struct EnsembleSource {
    pub nodes: Vec<ExecutionNode>,
    pub weight: f32,
    pub name: String,
}

/// An executable pipeline where all stages have been pre-linked.
#[derive(Debug)]
pub struct ExecutablePipeline {
    pub nodes: Vec<ExecutionNode>,
    pub fallback_nodes: Option<Vec<ExecutionNode>>,
}

/// Specialized error for pipeline execution.
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

/// Unified maturity rating levels for the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum MaturityRating {
    GE, // General Audience (0+)
    PG, // Parental Guidance (13+)
    M16, // Mature 16+
    M18, // Adults only 18+
}

impl MaturityRating {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GE" | "G" => Self::GE,
            "PG" | "PG-13" | "PG13" => Self::PG,
            "16" | "M16" => Self::M16,
            "18" | "M18" | "R" | "NC-17" | "NC17" => Self::M18,
            _ => Self::M18, // Strictest by default
        }
    }

    pub fn as_age(&self) -> i32 {
        match self {
            Self::GE => 0,
            Self::PG => 13,
            Self::M16 => 16,
            Self::M18 => 18,
        }
    }
}
