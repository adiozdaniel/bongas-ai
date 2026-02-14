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

  // ─── Error Context ──────────────────────────────────────────────────────────

  /// Contextual information attached to errors for tracing and debugging.
  #[derive(Debug, Clone)]
  pub struct ErrorContext {
      pub request_id: Option<String>,
      pub timestamp: Instant,
      pub component: &'static str,
      pub instance: Option<&'static str>,
      pub operation: Option<&'static str>,
      pub metadata: Vec<(&'static str, String)>,
  }

  impl ErrorContext {
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

      pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
          self.request_id = Some(id.into());
          self
      }

      pub fn with_instance(mut self, instance: &'static str) -> Self {
          self.instance = Some(instance);
          self
      }

      pub fn with_operation(mut self, operation: &'static str) -> Self {
          self.operation = Some(operation);
          self
      }

      pub fn with_metadata(mut self, key: &'static str, value: impl Into<String>) -> Self {
          self.metadata.push((key, value.into()));
          self
      }

      pub fn elapsed(&self) -> Duration {
          self.timestamp.elapsed()
      }

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
  #[derive(Debug, Clone)]
  pub struct PartialResult<T, E> {
      pub succeeded: Vec<T>,
      pub failed: Vec<(T, E)>,
  }

  impl<T, E> PartialResult<T, E> {
      pub fn new(succeeded: Vec<T>, failed: Vec<(T, E)>) -> Self {
          Self { succeeded, failed }
      }

      pub fn all_succeeded(items: Vec<T>) -> Self {
          Self { succeeded: items, failed: Vec::new() }
      }

      pub fn all_failed(items: Vec<(T, E)>) -> Self {
          Self { succeeded: Vec::new(), failed: items }
      }

      pub fn is_complete_success(&self) -> bool {
          self.failed.is_empty()
      }

      pub fn is_complete_failure(&self) -> bool {
          self.succeeded.is_empty()
      }

      pub fn is_partial(&self) -> bool {
          !self.succeeded.is_empty() && !self.failed.is_empty()
      }

      pub fn success_rate(&self) -> f64 {
          let total = self.succeeded.len() + self.failed.len();
          if total == 0 { 1.0 } else { self.succeeded.len() as f64 / total as f64 }
      }

      pub fn failed_items(self) -> Vec<T> {
          self.failed.into_iter().map(|(item, _)| item).collect()
      }
  }

  // ─── Domain Errors ──────────────────────────────────────────────────────────

  #[derive(Debug, Error)]
  pub enum RedisError {
      #[error("redis connection failed: {message}")]
      Connection { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
      #[error("redis serialization failed: {0}")]
      Serialization(String),
      #[error("redis pool exhausted")]
      PoolExhausted,
      #[error("redis operation timed out after {0:?}")]
      Timeout(Duration),
      #[error("redis command failed: {message}")]
      Command { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
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
  }

  #[derive(Debug, Error)]
  pub enum PostgresError {
      #[error("postgres query failed: {message}")]
      Query { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
      #[error("postgres pool exhausted")]
      PoolExhausted,
      #[error("postgres operation timed out after {0:?}")]
      Timeout(Duration),
      #[error("postgres migration failed: {0}")]
      Migration(String),
      #[error("postgres connection failed: {message}")]
      Connection { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
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

  #[derive(Debug, Error)]
  pub enum ClickHouseError {
      #[error("clickhouse query failed: {message}")]
      Query { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
      #[error("clickhouse connection failed: {message}")]
      Connection { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
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

  #[derive(Debug, Error)]
  pub enum KafkaError {
      #[error("kafka operation failed: {message}")]
      Operation { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
      #[error("kafka connection failed: {message}")]
      Connection { message: String },
      #[error("kafka operation timed out after {0:?}")]
      Timeout(Duration),
  }

  impl ErrorClassifier for KafkaError {
      fn classify(&self) -> ErrorClassification {
          match self {
              KafkaError::Operation { .. } => ErrorClassification::Transient,
              KafkaError::Connection { .. } => ErrorClassification::Transient,
              KafkaError::Timeout(_) => ErrorClassification::Timeout,
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
  pub enum PipelineError {
      #[error("pipeline stage failed: {0}")]
      StageFailed(String),
      #[error("pipeline configuration invalid: {0}")]
      InvalidConfig(String),
      #[error("pipeline operation timed out")]
      Timeout,
  }

  impl ErrorClassifier for PipelineError {
      fn classify(&self) -> ErrorClassification {
          match self {
              PipelineError::StageFailed(_) => ErrorClassification::Transient,
              PipelineError::InvalidConfig(_) => ErrorClassification::Permanent,
              PipelineError::Timeout => ErrorClassification::Timeout,
          }
      }
  }

  #[derive(Debug, Error)]
  pub enum ModelError {
      #[error("model not found: {0}")]
      NotFound(String),
      #[error("model inference failed: {0}")]
      InferenceFailed(String),
      #[error("model loading failed: {0}")]
      LoadFailed(String),
  }

  impl ErrorClassifier for ModelError {
      fn classify(&self) -> ErrorClassification {
          match self {
              ModelError::NotFound(_) => ErrorClassification::Permanent,
              ModelError::InferenceFailed(_) => ErrorClassification::Transient,
              ModelError::LoadFailed(_) => ErrorClassification::Transient,
          }
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

  #[derive(Debug, Error)]
  pub enum SecurityError {
      #[error("authentication failed: {0}")]
      AuthenticationFailed(String),
      #[error("authorization failed: {0}")]
      AuthorizationFailed(String),
      #[error("security validation failed: {0}")]
      ValidationFailed(String),
  }

  impl ErrorClassifier for SecurityError {
      fn classify(&self) -> ErrorClassification {
          match self {
              SecurityError::AuthenticationFailed(_) => ErrorClassification::Permanent,
              SecurityError::AuthorizationFailed(_) => ErrorClassification::Permanent,
              SecurityError::ValidationFailed(_) => ErrorClassification::Permanent,
          }
      }
  }

  #[derive(Debug, Error)]
  pub enum ExperimentError {
      #[error("experiment not found: {0}")]
      NotFound(String),
      #[error("experiment configuration invalid: {0}")]
      InvalidConfig(String),
  }

  impl ErrorClassifier for ExperimentError {
      fn classify(&self) -> ErrorClassification {
          match self {
              ExperimentError::NotFound(_) => ErrorClassification::Permanent,
              ExperimentError::InvalidConfig(_) => ErrorClassification::Permanent,
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

  // ─── Application-Level Composite ────────────────────────────────────────────

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
      #[error("internal error: {0}")]
      Internal(String),
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
              AppError::Internal(_) => ErrorClassification::Transient,
          }
      }
  }

  pub type AppResult<T> = Result<T, AppError>;
