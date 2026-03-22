use thiserror::Error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};

/// Pipeline error taxonomy for the Composite Resilience Pattern.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("pipeline configuration invalid: {0}")]
    InvalidConfig(String),
    #[error("pipeline stage not found in registry: {0}")]
    StageNotFound(String),
    #[error("pipeline stage failed: {stage} — {reason}")]
    StageFailed { stage: String, reason: String },
    #[error("pipeline fetch stage failed: {stage} — {reason}")]
    FetchFailed { stage: String, reason: String },
    #[error("pipeline database query failed in stage {stage}: {reason}")]
    DatabaseError { stage: String, reason: String },
    #[error("pipeline cache error in stage {stage}: {reason}")]
    CacheError { stage: String, reason: String },
    #[error("pipeline stage timed out after {timeout_ms}ms: {stage}")]
    StageTimeout { stage: String, timeout_ms: u64 },
    #[error("pipeline execution timed out after {timeout_ms}ms")]
    PipelineTimeout { timeout_ms: u64 },
    #[error("pipeline stage overloaded (queue depth {queue_depth}): {stage}")]
    StageOverloaded { stage: String, queue_depth: usize },
    #[error("circuit breaker rejected stage execution: {stage}")]
    CircuitOpen { stage: String },
    #[error("pipeline stage returned degraded result: {stage} — {reason}")]
    Degraded { stage: String, reason: String },
    #[error("pipeline fallback used for stage: {stage} — {reason}")]
    FallbackUsed { stage: String, reason: String },
    #[error("pipeline partial failure: {succeeded}/{total} stages completed")]
    PartialExecution { succeeded: usize, total: usize },
}

impl ErrorClassifier for PipelineError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PipelineError::InvalidConfig(_) | PipelineError::StageNotFound(_) => {
                ErrorClassification::Permanent
            }
            PipelineError::StageFailed { .. }
            | PipelineError::FetchFailed { .. }
            | PipelineError::DatabaseError { .. }
            | PipelineError::CacheError { .. } => ErrorClassification::Transient,
            PipelineError::StageTimeout { .. }
            | PipelineError::PipelineTimeout { .. } => ErrorClassification::Timeout,
            PipelineError::StageOverloaded { .. }
            | PipelineError::CircuitOpen { .. } => ErrorClassification::Overload,
            PipelineError::Degraded { .. }
            | PipelineError::FallbackUsed { .. } => ErrorClassification::Degraded,
            PipelineError::PartialExecution { .. } => ErrorClassification::PartialFailure,
        }
    }
}

/// ML model error taxonomy for the Composite Resilience Pattern.
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model not found: {0}")]
    NotFound(String),
    #[error("invalid model configuration: {0}")]
    InvalidConfig(String),
    #[error("model inference failed: {0}")]
    InferenceFailed(String),
    #[error("model loading failed: {0}")]
    LoadFailed(String),
    #[error("feature store error: {0}")]
    FeatureStore(String),
    #[error("embedding lookup failed: {0}")]
    EmbeddingLookup(String),
    #[error("model registry error: {0}")]
    Registry(String),
    #[error("model inference timed out after {timeout_ms}ms: {model}")]
    InferenceTimeout { model: String, timeout_ms: u64 },
    #[error("feature fetch timed out after {timeout_ms}ms")]
    FeatureTimeout { timeout_ms: u64 },
    #[error("model overloaded (queue depth {queue_depth}): {model}")]
    Overloaded { model: String, queue_depth: usize },
    #[error("circuit breaker rejected inference for model: {0}")]
    CircuitOpen(String),
    #[error("model returned degraded result: {reason}")]
    Degraded { reason: String },
    #[error("fallback result used: {reason}")]
    FallbackUsed { reason: String },
    #[error("batch inference partial failure: {succeeded}/{total} items")]
    PartialInference { succeeded: usize, total: usize },
}

impl ErrorClassifier for ModelError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ModelError::NotFound(_) | ModelError::InvalidConfig(_) => {
                ErrorClassification::Permanent
            }
            ModelError::InferenceFailed(_)
            | ModelError::LoadFailed(_)
            | ModelError::FeatureStore(_)
            | ModelError::EmbeddingLookup(_)
            | ModelError::Registry(_) => ErrorClassification::Transient,
            ModelError::InferenceTimeout { .. } | ModelError::FeatureTimeout { .. } => {
                ErrorClassification::Timeout
            }
            ModelError::Overloaded { .. } | ModelError::CircuitOpen(_) => {
                ErrorClassification::Overload
            }
            ModelError::Degraded { .. } | ModelError::FallbackUsed { .. } => {
                ErrorClassification::Degraded
            }
            ModelError::PartialInference { .. } => ErrorClassification::PartialFailure,
        }
    }
}

impl From<candle_core::Error> for ModelError {
    fn from(err: candle_core::Error) -> Self {
        ModelError::InferenceFailed(err.to_string())
    }
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("scenario not found: {0}")]
    NotFound(String),
    #[error("scenario execution failed: {0}")]
    ExecutionFailed(String),
    #[error("scenario configuration invalid: {0}")]
    InvalidConfig(String),
}

impl ErrorClassifier for ScenarioError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ScenarioError::NotFound(_) => ErrorClassification::Permanent,
            ScenarioError::ExecutionFailed(_) => ErrorClassification::Transient,
            ScenarioError::InvalidConfig(_) => ErrorClassification::Permanent,
        }
    }
}
