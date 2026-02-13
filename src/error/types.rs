//! Centralized error taxonomy for the Composite Resilience Pattern.
//!
//! Provides a layered error hierarchy that enables consistent error classification
//! across all resilience components (circuit breaker, retry, bulkhead). Each domain
//! module has a dedicated error type that implements `ErrorClassifier`, teaching the
//! resilience infrastructure how to respond without coupling to domain specifics.
//!
//! # Design Patterns
//! - **Strategy**: `ErrorClassifier` trait — each domain classifies its own errors.
//! - **Open/Closed**: New domains added by implementing `ErrorClassifier`, no
//!   changes needed in circuit breaker or retry logic.

use std::time::Duration;
use thiserror::Error;

// ─── Error Classification (Strategy Pattern) ────────────────────────────────

/// Determines how the resilience layer responds to an error.
///
/// The circuit breaker, retry logic, and bulkhead all branch on this enum.
/// Domain modules never interact with resilience internals directly — they
/// classify their errors, and the infrastructure acts accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClassification {
    /// Transient failure — safe to retry, counts toward circuit breaker threshold.
    Transient,
    /// Permanent failure — do NOT retry, does NOT trip the circuit breaker.
    Permanent,
    /// Timeout — retryable with different backoff, counts toward circuit breaker.
    Timeout,
    /// Overload — downstream is overwhelmed. Do NOT retry immediately.
    /// Counts toward circuit breaker. Signals bulkhead to shed load.
    Overload,
}

/// Classifies a domain error into a resilience-relevant category.
///
/// Every domain error type implements this trait so the circuit breaker and
/// retry logic can react without inspecting domain-specific variants.
pub trait ErrorClassifier {
    fn classify(&self) -> ErrorClassification;
}

// ─── Domain Errors ──────────────────────────────────────────────────────────

/// Redis cache operation errors.
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("redis connection failed: {0}")]
    Connection(String),
    #[error("redis serialization failed: {0}")]
    Serialization(String),
    #[error("redis pool exhausted")]
    PoolExhausted,
    #[error("redis operation timed out after {0:?}")]
    Timeout(Duration),
}

impl ErrorClassifier for RedisError {
    fn classify(&self) -> ErrorClassification {
        match self {
            RedisError::Connection(_) => ErrorClassification::Transient,
            RedisError::Serialization(_) => ErrorClassification::Permanent,
            RedisError::PoolExhausted => ErrorClassification::Overload,
            RedisError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}

/// PostgreSQL database operation errors.
#[derive(Debug, Error)]
pub enum PostgresError {
    #[error("postgres query failed: {0}")]
    Query(String),
    #[error("postgres pool exhausted")]
    PoolExhausted,
    #[error("postgres operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("postgres migration failed: {0}")]
    Migration(String),
}

impl ErrorClassifier for PostgresError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PostgresError::Query(_) => ErrorClassification::Transient,
            PostgresError::PoolExhausted => ErrorClassification::Overload,
            PostgresError::Timeout(_) => ErrorClassification::Timeout,
            PostgresError::Migration(_) => ErrorClassification::Permanent,
        }
    }
}

/// ClickHouse analytics database errors.
#[derive(Debug, Error)]
pub enum ClickHouseError {
    #[error("clickhouse query failed: {0}")]
    Query(String),
    #[error("clickhouse connection failed: {0}")]
    Connection(String),
    #[error("clickhouse operation timed out after {0:?}")]
    Timeout(Duration),
}

impl ErrorClassifier for ClickHouseError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ClickHouseError::Query(_) => ErrorClassification::Transient,
            ClickHouseError::Connection(_) => ErrorClassification::Transient,
            ClickHouseError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}

/// Kafka streaming errors.
#[derive(Debug, Error)]
pub enum KafkaError {
    #[error("kafka operation failed: {0}")]
    Client(String),
    #[error("kafka message deserialization failed: {0}")]
    Deserialization(String),
    #[error("kafka producer queue full")]
    QueueFull,
    #[error("kafka operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("kafka consumer group rebalance in progress")]
    Rebalancing,
}

impl ErrorClassifier for KafkaError {
    fn classify(&self) -> ErrorClassification {
        match self {
            KafkaError::Client(_) => ErrorClassification::Transient,
            KafkaError::Deserialization(_) => ErrorClassification::Permanent,
            KafkaError::QueueFull => ErrorClassification::Overload,
            KafkaError::Timeout(_) => ErrorClassification::Timeout,
            KafkaError::Rebalancing => ErrorClassification::Transient,
        }
    }
}

/// Cache layer errors (warming, invalidation, multi-tier coordination).
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache redis error: {0}")]
    Redis(RedisError),
    #[error("cache postgres error: {0}")]
    Postgres(PostgresError),
    #[error("cache key not found: {0}")]
    NotFound(String),
    #[error("cache warming failed: {0}")]
    WarmingFailed(String),
    #[error("cache invalidation failed: {0}")]
    InvalidationFailed(String),
    #[error("cache strategy not found for type: {0}")]
    StrategyNotFound(String),
}

impl ErrorClassifier for CacheError {
    fn classify(&self) -> ErrorClassification {
        match self {
            CacheError::Redis(e) => e.classify(),
            CacheError::Postgres(e) => e.classify(),
            CacheError::NotFound(_) => ErrorClassification::Permanent,
            CacheError::WarmingFailed(_) => ErrorClassification::Transient,
            CacheError::InvalidationFailed(_) => ErrorClassification::Transient,
            CacheError::StrategyNotFound(_) => ErrorClassification::Permanent,
        }
    }
}

/// Recommendation pipeline errors.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("pipeline stage '{stage}' failed: {reason}")]
    StageFailed { stage: String, reason: String },
    #[error("pipeline stage '{stage}' timed out after {timeout:?}")]
    StageTimeout { stage: String, timeout: Duration },
    #[error("pipeline configuration invalid: {0}")]
    Configuration(String),
    #[error("pipeline input validation failed: {0}")]
    Validation(String),
    #[error("pipeline empty result set after stage '{0}'")]
    EmptyResultSet(String),
}

impl ErrorClassifier for PipelineError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PipelineError::StageFailed { .. } => ErrorClassification::Transient,
            PipelineError::StageTimeout { .. } => ErrorClassification::Timeout,
            PipelineError::Configuration(_) => ErrorClassification::Permanent,
            PipelineError::Validation(_) => ErrorClassification::Permanent,
            PipelineError::EmptyResultSet(_) => ErrorClassification::Permanent,
        }
    }
}

/// ML model inference and lifecycle errors.
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model runtime error: {0}")]
    Runtime(String),
    #[error("model '{model_id}' not found")]
    NotFound { model_id: String },
    #[error("model inference timed out after {0:?}")]
    Timeout(Duration),
    #[error("model input shape mismatch: expected {expected}, got {actual}")]
    ShapeMismatch { expected: String, actual: String },
    #[error("model loading failed: {0}")]
    LoadFailed(String),
    #[error("model feature extraction failed: {0}")]
    FeatureExtraction(String),
}

impl ErrorClassifier for ModelError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ModelError::Runtime(_) => ErrorClassification::Transient,
            ModelError::NotFound { .. } => ErrorClassification::Permanent,
            ModelError::Timeout(_) => ErrorClassification::Timeout,
            ModelError::ShapeMismatch { .. } => ErrorClassification::Permanent,
            ModelError::LoadFailed(_) => ErrorClassification::Transient,
            ModelError::FeatureExtraction(_) => ErrorClassification::Permanent,
        }
    }
}

/// Scenario execution and selection errors.
#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("scenario '{slug}' not found")]
    NotFound { slug: String },
    #[error("scenario execution failed: {0}")]
    ExecutionFailed(String),
    #[error("scenario validation failed: {0}")]
    Validation(String),
    #[error("scenario configuration invalid: {0}")]
    Configuration(String),
    #[error("scenario optimization failed: {0}")]
    OptimizationFailed(String),
}

impl ErrorClassifier for ScenarioError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ScenarioError::NotFound { .. } => ErrorClassification::Permanent,
            ScenarioError::ExecutionFailed(_) => ErrorClassification::Transient,
            ScenarioError::Validation(_) => ErrorClassification::Permanent,
            ScenarioError::Configuration(_) => ErrorClassification::Permanent,
            ScenarioError::OptimizationFailed(_) => ErrorClassification::Transient,
        }
    }
}

/// Security and authentication errors.
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("license validation failed: {0}")]
    LicenseInvalid(String),
    #[error("integrity check failed: {0}")]
    IntegrityViolation(String),
    #[error("hardware fingerprint mismatch")]
    HardwareMismatch,
    #[error("anti-debug check triggered: {0}")]
    DebugDetected(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("authorization denied: {0}")]
    Forbidden(String),
}

impl ErrorClassifier for SecurityError {
    fn classify(&self) -> ErrorClassification {
        match self {
            SecurityError::LicenseInvalid(_) => ErrorClassification::Permanent,
            SecurityError::IntegrityViolation(_) => ErrorClassification::Permanent,
            SecurityError::HardwareMismatch => ErrorClassification::Permanent,
            SecurityError::DebugDetected(_) => ErrorClassification::Permanent,
            SecurityError::AuthFailed(_) => ErrorClassification::Permanent,
            SecurityError::Forbidden(_) => ErrorClassification::Permanent,
        }
    }
}

/// Experiment and A/B testing errors.
#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("experiment '{experiment_id}' not found")]
    NotFound { experiment_id: String },
    #[error("experiment variant selection failed: {0}")]
    VariantSelection(String),
    #[error("experiment metric collection failed: {0}")]
    MetricCollection(String),
    #[error("experiment analysis failed: {0}")]
    Analysis(String),
    #[error("experiment configuration invalid: {0}")]
    Configuration(String),
}

impl ErrorClassifier for ExperimentError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ExperimentError::NotFound { .. } => ErrorClassification::Permanent,
            ExperimentError::VariantSelection(_) => ErrorClassification::Transient,
            ExperimentError::MetricCollection(_) => ErrorClassification::Transient,
            ExperimentError::Analysis(_) => ErrorClassification::Transient,
            ExperimentError::Configuration(_) => ErrorClassification::Permanent,
        }
    }
}

/// HTTP middleware errors.
#[derive(Debug, Error)]
pub enum MiddlewareError {
    #[error("rate limit exceeded for {0}")]
    RateLimited(String),
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    #[error("compression failed: {0}")]
    Compression(String),
    #[error("CORS rejected origin: {0}")]
    CorsRejected(String),
    #[error("middleware chain error: {0}")]
    Chain(String),
}

impl ErrorClassifier for MiddlewareError {
    fn classify(&self) -> ErrorClassification {
        match self {
            MiddlewareError::RateLimited(_) => ErrorClassification::Overload,
            MiddlewareError::Timeout(_) => ErrorClassification::Timeout,
            MiddlewareError::Compression(_) => ErrorClassification::Permanent,
            MiddlewareError::CorsRejected(_) => ErrorClassification::Permanent,
            MiddlewareError::Chain(_) => ErrorClassification::Transient,
        }
    }
}

/// Prometheus metrics registration and collection errors.
#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("metrics registration failed: {0}")]
    Registration(String),
    #[error("metrics collection failed: {0}")]
    Collection(String),
    #[error("metrics encoding failed: {0}")]
    Encoding(String),
}

impl ErrorClassifier for MetricsError {
    fn classify(&self) -> ErrorClassification {
        match self {
            MetricsError::Registration(_) => ErrorClassification::Permanent,
            MetricsError::Collection(_) => ErrorClassification::Transient,
            MetricsError::Encoding(_) => ErrorClassification::Permanent,
        }
    }
}

// ─── Application Error (Composite) ─────────────────────────────────────────

/// Top-level application error that wraps all domain errors.
///
/// Every domain error converts into `AppError` via `From`, giving the
/// resilience layer and API layer a single type to work with.
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error(transparent)]
    Postgres(#[from] PostgresError),
    #[error(transparent)]
    ClickHouse(#[from] ClickHouseError),
    #[error(transparent)]
    Kafka(#[from] KafkaError),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Scenario(#[from] ScenarioError),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error(transparent)]
    Experiment(#[from] ExperimentError),
    #[error(transparent)]
    Middleware(#[from] MiddlewareError),
    #[error(transparent)]
    Metrics(#[from] MetricsError),
}

impl ErrorClassifier for AppError {
    fn classify(&self) -> ErrorClassification {
        match self {
            AppError::Redis(e) => e.classify(),
            AppError::Postgres(e) => e.classify(),
            AppError::ClickHouse(e) => e.classify(),
            AppError::Kafka(e) => e.classify(),
            AppError::Cache(e) => e.classify(),
            AppError::Pipeline(e) => e.classify(),
            AppError::Model(e) => e.classify(),
            AppError::Scenario(e) => e.classify(),
            AppError::Security(e) => e.classify(),
            AppError::Experiment(e) => e.classify(),
            AppError::Middleware(e) => e.classify(),
            AppError::Metrics(e) => e.classify(),
        }
    }
}

/// Convenience type alias for results using `AppError`.
pub type AppResult<T> = Result<T, AppError>;
