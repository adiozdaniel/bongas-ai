//! Centralized error taxonomy for the Composite Resilience Pattern.
//!
//! Provides a layered error hierarchy that enables consistent error classification
//! across all resilience components (circuit breaker, retry, bulkhead). Each domain
//! module has a dedicated error type that implements `ErrorClassifier`, teaching the
//! resilience infrastructure how to respond without coupling to domain specifics.
//!
//! # Netflix Resilience Features
//! - **Classification**: Transient, Permanent, Timeout, Overload, Degraded, PartialFailure
//! - **Retry Hints**: Backoff strategy, max retries, retry-after duration
//! - **Error Context**: Request ID, timestamp, component origin for tracing
//! - **Cause Chain**: Wrapped source errors with `#[source]` for debugging
//!
//! # Design Patterns
//! - **Strategy**: `ErrorClassifier` trait — each domain classifies its own errors.
//! - **Open/Closed**: New domains added by implementing `ErrorClassifier`, no
//!   changes needed in circuit breaker or retry logic.
//! - **Composite**: `AppError` aggregates all domain errors into a single type.

use std::time::{Duration, Instant};
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
    /// Degraded — operation partially succeeded or used fallback.
    /// May retry for full result, does NOT trip circuit breaker.
    Degraded,
    /// Partial failure — some items succeeded, some failed.
    /// Retry only failed items if possible.
    PartialFailure,
}

impl ErrorClassification {
    /// Returns true if this error type should be retried.
    #[inline]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::PartialFailure
        )
    }

    /// Returns true if this error should count toward circuit breaker threshold.
    #[inline]
    pub fn should_trip(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Transient
                | ErrorClassification::Timeout
                | ErrorClassification::Overload
        )
    }

    /// Returns true if retry should be delayed (backoff required).
    #[inline]
    pub fn requires_backoff(&self) -> bool {
        matches!(
            self,
            ErrorClassification::Timeout | ErrorClassification::Overload
        )
    }
}

// ─── Retry Hints ────────────────────────────────────────────────────────────

/// Backoff strategy for retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BackoffStrategy {
    /// No delay between retries.
    None,
    /// Fixed delay between retries.
    #[default]
    Fixed,
    /// Exponential backoff with optional jitter.
    Exponential,
    /// Linear increase in delay.
    Linear,
}

/// Hints for retry behavior, provided by the error source.
///
/// Resilience infrastructure uses these hints to make intelligent retry decisions
/// without hardcoding domain-specific logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryHint {
    /// Suggested backoff strategy.
    pub strategy: BackoffStrategy,
    /// Base delay for backoff calculation.
    pub base_delay: Duration,
    /// Maximum number of retries (None = use default).
    pub max_retries: Option<u32>,
    /// Absolute time after which retry is allowed (e.g., from Retry-After header).
    pub retry_after: Option<Duration>,
    /// Whether to add jitter to prevent thundering herd.
    pub add_jitter: bool,
}

impl Default for RetryHint {
    fn default() -> Self {
        Self {
            strategy: BackoffStrategy::Exponential,
            base_delay: Duration::from_millis(100),
            max_retries: Some(3),
            retry_after: None,
            add_jitter: true,
        }
    }
}

impl RetryHint {
    /// Create a hint for immediate retry (transient errors).
    pub fn immediate() -> Self {
        Self {
            strategy: BackoffStrategy::None,
            base_delay: Duration::ZERO,
            max_retries: Some(3),
            retry_after: None,
            add_jitter: false,
        }
    }

    /// Create a hint for exponential backoff (default).
    pub fn exponential(base_delay: Duration) -> Self {
        Self {
            strategy: BackoffStrategy::Exponential,
            base_delay,
            max_retries: Some(3),
            retry_after: None,
            add_jitter: true,
        }
    }

    /// Create a hint with a specific retry-after duration (e.g., from 429 response).
    pub fn after(duration: Duration) -> Self {
        Self {
            strategy: BackoffStrategy::Fixed,
            base_delay: duration,
            max_retries: Some(1),
            retry_after: Some(duration),
            add_jitter: false,
        }
    }

    /// Create a hint indicating no retry should be attempted.
    pub fn no_retry() -> Self {
        Self {
            strategy: BackoffStrategy::None,
            base_delay: Duration::ZERO,
            max_retries: Some(0),
            retry_after: None,
            add_jitter: false,
        }
    }

    /// Set maximum retries.
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = Some(max);
        self
    }

    /// Enable or disable jitter.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.add_jitter = jitter;
        self
    }
}

// ─── Error Context ──────────────────────────────────────────────────────────

/// Contextual information attached to errors for tracing and debugging.
///
/// Carries enough information to correlate errors across distributed systems
/// without exposing internal implementation details.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Unique identifier for the request that caused this error.
    pub request_id: Option<String>,
    /// When the error occurred.
    pub timestamp: Instant,
    /// Component that originated the error (e.g., "redis", "postgres").
    pub component: &'static str,
    /// Optional sub-component or instance (e.g., "user_cache", "primary").
    pub instance: Option<&'static str>,
    /// Operation that was being performed (e.g., "get", "set", "query").
    pub operation: Option<&'static str>,
    /// Additional key-value metadata.
    pub metadata: Vec<(&'static str, String)>,
}

impl ErrorContext {
    /// Create a new error context for a component.
    pub fn new(component: &'static str) -> Self {
        Self {
            request_id: None,
            timestamp: Instant::now(),
            component,
            instance: None,
            operation: None,
            metadata: Vec::new(),
        }
    }

    /// Set the request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set the instance name.
    pub fn with_instance(mut self, instance: &'static str) -> Self {
        self.instance = Some(instance);
        self
    }

    /// Set the operation name.
    pub fn with_operation(mut self, operation: &'static str) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Add metadata key-value pair.
    pub fn with_metadata(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.metadata.push((key, value.into()));
        self
    }

    /// Get the elapsed time since the error occurred.
    pub fn elapsed(&self) -> Duration {
        self.timestamp.elapsed()
    }

    /// Format as a label suitable for metrics.
    pub fn label(&self) -> String {
        match self.instance {
            Some(inst) => format!("{}_{}", self.component, inst),
            None => self.component.to_string(),
        }
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new("unknown")
    }
}

// ─── Partial Failure Support ────────────────────────────────────────────────

/// Result of a batch operation where some items may have failed.
///
/// Enables fine-grained retry of only failed items instead of all-or-nothing.
#[derive(Debug, Clone)]
pub struct PartialResult<T, E> {
    /// Items that succeeded.
    pub succeeded: Vec<T>,
    /// Items that failed with their errors.
    pub failed: Vec<(T, E)>,
}

impl<T, E> PartialResult<T, E> {
    /// Create a new partial result.
    pub fn new(succeeded: Vec<T>, failed: Vec<(T, E)>) -> Self {
        Self { succeeded, failed }
    }

    /// Create a fully successful result.
    pub fn all_succeeded(items: Vec<T>) -> Self {
        Self {
            succeeded: items,
            failed: Vec::new(),
        }
    }

    /// Create a fully failed result.
    pub fn all_failed(items: Vec<(T, E)>) -> Self {
        Self {
            succeeded: Vec::new(),
            failed: items,
        }
    }

    /// Returns true if all items succeeded.
    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Returns true if all items failed.
    pub fn is_complete_failure(&self) -> bool {
        self.succeeded.is_empty()
    }

    /// Returns true if some items succeeded and some failed.
    pub fn is_partial(&self) -> bool {
        !self.succeeded.is_empty() && !self.failed.is_empty()
    }

    /// Get the success rate as a fraction.
    pub fn success_rate(&self) -> f64 {
        let total = self.succeeded.len() + self.failed.len();
        if total == 0 {
            1.0
        } else {
            self.succeeded.len() as f64 / total as f64
        }
    }

    /// Extract only the failed items for retry.
    pub fn failed_items(self) -> Vec<T> {
        self.failed.into_iter().map(|(item, _)| item).collect()
    }
}

// ─── Error Classifier Trait ─────────────────────────────────────────────────

/// Classifies a domain error into a resilience-relevant category.
///
/// Every domain error type implements this trait so the circuit breaker and
/// retry logic can react without inspecting domain-specific variants.
///
/// # Default Implementations
/// The trait provides default implementations for helper methods that derive
/// from `classify()`. Override only if domain-specific behavior is needed.
pub trait ErrorClassifier {
    /// Classify this error for resilience decision-making.
    fn classify(&self) -> ErrorClassification;

    /// Returns true if this error should be retried.
    ///
    /// Default: delegates to `ErrorClassification::is_retriable()`.
    #[inline]
    fn is_retriable(&self) -> bool {
        self.classify().is_retriable()
    }

    /// Returns true if this error should count toward circuit breaker threshold.
    ///
    /// Default: delegates to `ErrorClassification::should_trip()`.
    #[inline]
    fn should_trip(&self) -> bool {
        self.classify().should_trip()
    }

    /// Returns retry hints for this error.
    ///
    /// Default: returns a hint based on classification.
    fn retry_hint(&self) -> RetryHint {
        match self.classify() {
            ErrorClassification::Transient => RetryHint::exponential(Duration::from_millis(100)),
            ErrorClassification::Timeout => RetryHint::exponential(Duration::from_millis(500)),
            ErrorClassification::Overload => RetryHint::exponential(Duration::from_secs(1)),
            ErrorClassification::Permanent => RetryHint::no_retry(),
            ErrorClassification::Degraded => RetryHint::exponential(Duration::from_millis(200)),
            ErrorClassification::PartialFailure => RetryHint::immediate(),
        }
    }

    /// Returns error context if available.
    ///
    /// Default: returns None. Override to provide tracing context.
    fn context(&self) -> Option<&ErrorContext> {
        None
    }
}

// ─── Domain Errors ──────────────────────────────────────────────────────────

/// Redis cache operation errors.
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("redis connection failed: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("redis serialization failed: {0}")]
    Serialization(String),
    #[error("redis pool exhausted")]
    PoolExhausted,
    #[error("redis operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("redis command failed: {message}")]
    Command {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ErrorClassifier for RedisError {
    fn classify(&self) -> ErrorClassification {
        match self {
            RedisError::Connection { .. } => ErrorClassification::Transient,
            RedisError::Serialization(_) => ErrorClassification::Permanent,
            RedisError::PoolExhausted => ErrorClassification::Overload,
            RedisError::Timeout(_) => ErrorClassification::Timeout,
            RedisError::Command { .. } => ErrorClassification::Transient,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            RedisError::PoolExhausted => RetryHint::exponential(Duration::from_secs(1))
                .with_max_retries(5),
            RedisError::Timeout(duration) => RetryHint::exponential(*duration)
                .with_max_retries(2),
            _ => RetryHint::default(),
        }
    }
}

/// PostgreSQL database operation errors.
#[derive(Debug, Error)]
pub enum PostgresError {
    #[error("postgres query failed: {message}")]
    Query {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("postgres pool exhausted")]
    PoolExhausted,
    #[error("postgres operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("postgres migration failed: {0}")]
    Migration(String),
    #[error("postgres connection failed: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("postgres constraint violation: {0}")]
    ConstraintViolation(String),
}

impl ErrorClassifier for PostgresError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PostgresError::Query { .. } => ErrorClassification::Transient,
            PostgresError::PoolExhausted => ErrorClassification::Overload,
            PostgresError::Timeout(_) => ErrorClassification::Timeout,
            PostgresError::Migration(_) => ErrorClassification::Permanent,
            PostgresError::Connection { .. } => ErrorClassification::Transient,
            PostgresError::ConstraintViolation(_) => ErrorClassification::Permanent,
        }
    }
}

/// ClickHouse analytics database errors.
#[derive(Debug, Error)]
pub enum ClickHouseError {
    #[error("clickhouse query failed: {message}")]
    Query {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("clickhouse connection failed: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("clickhouse operation timed out after {0:?}")]
    Timeout(Duration),
}

impl ErrorClassifier for ClickHouseError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ClickHouseError::Query { .. } => ErrorClassification::Transient,
            ClickHouseError::Connection { .. } => ErrorClassification::Transient,
            ClickHouseError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}

/// Kafka streaming errors.
#[derive(Debug, Error)]
pub enum KafkaError {
    #[error("kafka operation failed: {message}")]
    Client {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
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
            KafkaError::Client { .. } => ErrorClassification::Transient,
            KafkaError::Deserialization(_) => ErrorClassification::Permanent,
            KafkaError::QueueFull => ErrorClassification::Overload,
            KafkaError::Timeout(_) => ErrorClassification::Timeout,
            KafkaError::Rebalancing => ErrorClassification::Transient,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            KafkaError::Rebalancing => RetryHint::exponential(Duration::from_secs(2))
                .with_max_retries(10),
            KafkaError::QueueFull => RetryHint::exponential(Duration::from_millis(500))
                .with_max_retries(5),
            _ => RetryHint::default(),
        }
    }
}

/// Cache layer errors (warming, invalidation, multi-tier coordination).
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("cache postgres error: {0}")]
    Postgres(#[from] PostgresError),
    #[error("cache key not found: {0}")]
    NotFound(String),
    #[error("cache warming failed: {0}")]
    WarmingFailed(String),
    #[error("cache invalidation failed: {0}")]
    InvalidationFailed(String),
    #[error("cache strategy not found for type: {0}")]
    StrategyNotFound(String),
    #[error("cache returned stale data (fallback used)")]
    StaleData {
        /// Age of the stale data.
        age: Duration,
    },
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
            CacheError::StaleData { .. } => ErrorClassification::Degraded,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            CacheError::Redis(e) => e.retry_hint(),
            CacheError::Postgres(e) => e.retry_hint(),
            CacheError::StaleData { .. } => RetryHint::exponential(Duration::from_secs(5)),
            _ => RetryHint::default(),
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
    #[error("pipeline partial failure: {succeeded} succeeded, {failed} failed")]
    PartialFailure { succeeded: usize, failed: usize },
}

impl ErrorClassifier for PipelineError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PipelineError::StageFailed { .. } => ErrorClassification::Transient,
            PipelineError::StageTimeout { .. } => ErrorClassification::Timeout,
            PipelineError::Configuration(_) => ErrorClassification::Permanent,
            PipelineError::Validation(_) => ErrorClassification::Permanent,
            PipelineError::EmptyResultSet(_) => ErrorClassification::Permanent,
            PipelineError::PartialFailure { .. } => ErrorClassification::PartialFailure,
        }
    }
}

/// ML model inference and lifecycle errors.
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model runtime error: {message}")]
    Runtime {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
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
    #[error("model returned low-confidence result (fallback recommended)")]
    LowConfidence { confidence: f32, threshold: f32 },
}

impl ErrorClassifier for ModelError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ModelError::Runtime { .. } => ErrorClassification::Transient,
            ModelError::NotFound { .. } => ErrorClassification::Permanent,
            ModelError::Timeout(_) => ErrorClassification::Timeout,
            ModelError::ShapeMismatch { .. } => ErrorClassification::Permanent,
            ModelError::LoadFailed(_) => ErrorClassification::Transient,
            ModelError::FeatureExtraction(_) => ErrorClassification::Permanent,
            ModelError::LowConfidence { .. } => ErrorClassification::Degraded,
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
        // Security errors are always permanent - never retry
        ErrorClassification::Permanent
    }

    fn retry_hint(&self) -> RetryHint {
        RetryHint::no_retry()
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
    #[error("rate limit exceeded for {client}")]
    RateLimited {
        client: String,
        retry_after: Option<Duration>,
    },
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
            MiddlewareError::RateLimited { .. } => ErrorClassification::Overload,
            MiddlewareError::Timeout(_) => ErrorClassification::Timeout,
            MiddlewareError::Compression(_) => ErrorClassification::Permanent,
            MiddlewareError::CorsRejected(_) => ErrorClassification::Permanent,
            MiddlewareError::Chain(_) => ErrorClassification::Transient,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            MiddlewareError::RateLimited { retry_after, .. } => {
                match retry_after {
                    Some(duration) => RetryHint::after(*duration),
                    None => RetryHint::exponential(Duration::from_secs(1)),
                }
            }
            _ => RetryHint::default(),
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

    fn retry_hint(&self) -> RetryHint {
        match self {
            AppError::Redis(e) => e.retry_hint(),
            AppError::Postgres(e) => e.retry_hint(),
            AppError::ClickHouse(e) => e.retry_hint(),
            AppError::Kafka(e) => e.retry_hint(),
            AppError::Cache(e) => e.retry_hint(),
            AppError::Pipeline(e) => e.retry_hint(),
            AppError::Model(e) => e.retry_hint(),
            AppError::Scenario(e) => e.retry_hint(),
            AppError::Security(e) => e.retry_hint(),
            AppError::Experiment(e) => e.retry_hint(),
            AppError::Middleware(e) => e.retry_hint(),
            AppError::Metrics(e) => e.retry_hint(),
        }
    }
}

/// Convenience type alias for results using `AppError`.
pub type AppResult<T> = Result<T, AppError>;
