//! Pipeline execution error types.

use super::models::StageDataKind;

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
