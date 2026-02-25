use std::time::Duration;
use thiserror::Error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};

#[derive(Debug, Error)]
pub enum IngestionError {
    #[error("ingestion source unavailable: {0}")]
    SourceUnavailable(String),
    #[error("activity processing failed ({activity_type}): {message}")]
    ProcessingFailed { activity_type: String, message: String },
    #[error("all ingestion sources degraded")]
    AllSourcesDegraded,
    #[error("ingestion operation timed out after {0:?}")]
    Timeout(Duration),
}

impl ErrorClassifier for IngestionError {
    fn classify(&self) -> ErrorClassification {
        match self {
            IngestionError::SourceUnavailable(_) => ErrorClassification::Transient,
            IngestionError::ProcessingFailed { .. } => ErrorClassification::Transient,
            IngestionError::AllSourcesDegraded => ErrorClassification::Degraded,
            IngestionError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache miss: {0}")]
    Miss(String),
    #[error("cache operation failed: {0}")]
    Operation(String),
    #[error("cache serialization failed: {0}")]
    Serialization(String),
}

impl ErrorClassifier for CacheError {
    fn classify(&self) -> ErrorClassification {
        match self {
            CacheError::Miss(_) => ErrorClassification::Degraded,
            CacheError::Operation(_) => ErrorClassification::Transient,
            CacheError::Serialization(_) => ErrorClassification::Permanent,
        }
    }
}

#[derive(Debug, Error)]
pub enum MiddlewareError {
    #[error("middleware failed: {0}")]
    Failed(String),
    #[error("rate limit exceeded")]
    RateLimitExceeded,
}

impl ErrorClassifier for MiddlewareError {
    fn classify(&self) -> ErrorClassification {
        match self {
            MiddlewareError::Failed(_) => ErrorClassification::Transient,
            MiddlewareError::RateLimitExceeded => ErrorClassification::Overload,
        }
    }
}

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("metrics collection failed: {0}")]
    CollectionFailed(String),
    #[error("metrics export failed: {0}")]
    ExportFailed(String),
}

impl ErrorClassifier for MetricsError {
    fn classify(&self) -> ErrorClassification {
        match self {
            MetricsError::CollectionFailed(_) => ErrorClassification::Degraded,
            MetricsError::ExportFailed(_) => ErrorClassification::Degraded,
        }
    }
}
