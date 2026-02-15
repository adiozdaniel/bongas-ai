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
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
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

  impl From<redis::RedisError> for RedisError {
      fn from(err: redis::RedisError) -> Self {
          if err.is_timeout() {
              RedisError::Timeout(Duration::from_secs(30))
          } else if err.is_connection_refusal() || err.is_io_error() {
              RedisError::Connection {
                  message: err.to_string(),
                  source: Some(Box::new(err)),
              }
          } else {
              RedisError::Command {
                  message: err.to_string(),
                  source: Some(Box::new(err)),
              }
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

  /// Pipeline error taxonomy for the Composite Resilience Pattern.
  ///
  /// Each variant maps to an `ErrorClassification` that drives per-stage
  /// circuit breaker, retry, and fallback behavior. Analytics are recorded
  /// at the executor level via `PerformanceMetrics`.
  #[derive(Debug, Error)]
  pub enum PipelineError {
      // ── Permanent (no retry, no breaker trip) ──────────────────────────────
      #[error("pipeline configuration invalid: {0}")]
      InvalidConfig(String),
      #[error("pipeline stage not found in registry: {0}")]
      StageNotFound(String),

      // ── Transient (retry with backoff, trips breaker) ─────────────────────
      #[error("pipeline stage failed: {stage} — {reason}")]
      StageFailed { stage: String, reason: String },
      #[error("pipeline fetch stage failed: {stage} — {reason}")]
      FetchFailed { stage: String, reason: String },
      #[error("pipeline database query failed in stage {stage}: {reason}")]
      DatabaseError { stage: String, reason: String },
      #[error("pipeline cache error in stage {stage}: {reason}")]
      CacheError { stage: String, reason: String },

      // ── Timeout (retry with longer backoff, trips breaker) ────────────────
      #[error("pipeline stage timed out after {timeout_ms}ms: {stage}")]
      StageTimeout { stage: String, timeout_ms: u64 },
      #[error("pipeline execution timed out after {timeout_ms}ms")]
      PipelineTimeout { timeout_ms: u64 },

      // ── Overload (no immediate retry, trips breaker, shed load) ───────────
      #[error("pipeline stage overloaded (queue depth {queue_depth}): {stage}")]
      StageOverloaded { stage: String, queue_depth: usize },
      #[error("circuit breaker rejected stage execution: {stage}")]
      CircuitOpen { stage: String },

      // ── Degraded (may retry for full result, does NOT trip breaker) ───────
      #[error("pipeline stage returned degraded result: {stage} — {reason}")]
      Degraded { stage: String, reason: String },
      #[error("pipeline fallback used for stage: {stage} — {reason}")]
      FallbackUsed { stage: String, reason: String },

      // ── Partial failure (retry only failed items) ─────────────────────────
      #[error("pipeline partial failure: {succeeded}/{total} stages completed")]
      PartialExecution { succeeded: usize, total: usize },
  }

  impl ErrorClassifier for PipelineError {
      fn classify(&self) -> ErrorClassification {
          match self {
              // Permanent — caller should not retry
              PipelineError::InvalidConfig(_) | PipelineError::StageNotFound(_) => {
                  ErrorClassification::Permanent
              }
              // Transient — safe to retry, counts toward breaker
              PipelineError::StageFailed { .. }
              | PipelineError::FetchFailed { .. }
              | PipelineError::DatabaseError { .. }
              | PipelineError::CacheError { .. } => ErrorClassification::Transient,
              // Timeout — retryable with backoff, counts toward breaker
              PipelineError::StageTimeout { .. }
              | PipelineError::PipelineTimeout { .. } => ErrorClassification::Timeout,
              // Overload — do NOT retry immediately, trips breaker
              PipelineError::StageOverloaded { .. }
              | PipelineError::CircuitOpen { .. } => ErrorClassification::Overload,
              // Degraded — partial success, does NOT trip breaker
              PipelineError::Degraded { .. }
              | PipelineError::FallbackUsed { .. } => ErrorClassification::Degraded,
              // Partial failure — retry only failed stages
              PipelineError::PartialExecution { .. } => ErrorClassification::PartialFailure,
          }
      }
  }

  /// ML model error taxonomy for the Composite Resilience Pattern.
  ///
  /// Each variant maps to an `ErrorClassification` that drives circuit breaker,
  /// retry, and bulkhead behavior. Analytics are recorded at the call site via
  /// `PerformanceMetrics` before the error propagates.
  #[derive(Debug, Error)]
  pub enum ModelError {
      // ── Permanent (no retry, no breaker trip) ──────────────────────────────
      #[error("model not found: {0}")]
      NotFound(String),
      #[error("invalid model configuration: {0}")]
      InvalidConfig(String),

      // ── Transient (retry with backoff, trips breaker) ─────────────────────
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

      // ── Timeout (retry with longer backoff, trips breaker) ────────────────
      #[error("model inference timed out after {timeout_ms}ms: {model}")]
      InferenceTimeout { model: String, timeout_ms: u64 },
      #[error("feature fetch timed out after {timeout_ms}ms")]
      FeatureTimeout { timeout_ms: u64 },

      // ── Overload (no immediate retry, trips breaker, shed load) ───────────
      #[error("model overloaded (queue depth {queue_depth}): {model}")]
      Overloaded { model: String, queue_depth: usize },
      #[error("circuit breaker rejected inference for model: {0}")]
      CircuitOpen(String),

      // ── Degraded (may retry for full result, does NOT trip breaker) ───────
      #[error("model returned degraded result: {reason}")]
      Degraded { reason: String },
      #[error("fallback result used: {reason}")]
      FallbackUsed { reason: String },

      // ── Partial failure (retry only failed items) ─────────────────────────
      #[error("batch inference partial failure: {succeeded}/{total} items")]
      PartialInference { succeeded: usize, total: usize },
  }

  impl ErrorClassifier for ModelError {
      fn classify(&self) -> ErrorClassification {
          match self {
              // Permanent — caller should not retry
              ModelError::NotFound(_) | ModelError::InvalidConfig(_) => {
                  ErrorClassification::Permanent
              }
              // Transient — safe to retry, counts toward breaker
              ModelError::InferenceFailed(_)
              | ModelError::LoadFailed(_)
              | ModelError::FeatureStore(_)
              | ModelError::EmbeddingLookup(_)
              | ModelError::Registry(_) => ErrorClassification::Transient,
              // Timeout — retryable with backoff, counts toward breaker
              ModelError::InferenceTimeout { .. } | ModelError::FeatureTimeout { .. } => {
                  ErrorClassification::Timeout
              }
              // Overload — do NOT retry immediately, trips breaker
              ModelError::Overloaded { .. } | ModelError::CircuitOpen(_) => {
                  ErrorClassification::Overload
              }
              // Degraded — partial success, does NOT trip breaker
              ModelError::Degraded { .. } | ModelError::FallbackUsed { .. } => {
                  ErrorClassification::Degraded
              }
              // Partial failure — retry only failed items
              ModelError::PartialInference { .. } => ErrorClassification::PartialFailure,
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
      // Permanent errors (no retry, no breaker trip)
      #[error("license invalid: {0}")]
      LicenseInvalid(String),
      #[error("license revoked: {0}")]
      LicenseRevoked(String),
      #[error("hardware mismatch: {0}")]
      HardwareMismatch(String),
      #[error("binary tampered: {0}")]
      BinaryTampered(String),
      #[error("debugger detected")]
      DebuggerDetected,
      #[error("analysis tool detected: {0}")]
      AnalysisToolDetected(String),

      // Transient errors (retry with backoff, trips breaker)
      #[error("server validation failed: {reason}")]
      ServerValidationFailed { reason: String },
      #[error("revocation check failed: {reason}")]
      RevocationCheckFailed { reason: String },
      #[error("hardware fingerprint failed: {reason}")]
      HardwareFingerprintFailed { reason: String },

      // Timeout errors (retry with longer backoff, trips breaker)
      #[error("server timeout after {timeout_ms}ms")]
      ServerTimeout { timeout_ms: u64 },
      #[error("heartbeat timeout after {timeout_ms}ms")]
      HeartbeatTimeout { timeout_ms: u64 },

      // Overload errors (no immediate retry, trips breaker, shed load)
      #[error("circuit breaker open for security layer")]
      CircuitOpen,
      #[error("validation overloaded (queue depth {queue_depth})")]
      ValidationOverloaded { queue_depth: usize },

      // Degraded errors (may retry for full result, does NOT trip breaker)
      #[error("degraded security check: {layer} - {reason}")]
      Degraded { layer: String, reason: String },
      #[error("fallback used for security layer: {layer} - {reason}")]
      FallbackUsed { layer: String, reason: String },
  }

  impl ErrorClassifier for SecurityError {
      fn classify(&self) -> ErrorClassification {
          match self {
              // Permanent — no retry, no breaker trip
              SecurityError::LicenseInvalid(_)
              | SecurityError::LicenseRevoked(_)
              | SecurityError::HardwareMismatch(_)
              | SecurityError::BinaryTampered(_)
              | SecurityError::DebuggerDetected
              | SecurityError::AnalysisToolDetected(_) => ErrorClassification::Permanent,

              // Transient — safe to retry, counts toward breaker
              SecurityError::ServerValidationFailed { .. }
              | SecurityError::RevocationCheckFailed { .. }
              | SecurityError::HardwareFingerprintFailed { .. } => ErrorClassification::Transient,

              // Timeout — retryable with backoff, counts toward breaker
              SecurityError::ServerTimeout { .. }
              | SecurityError::HeartbeatTimeout { .. } => ErrorClassification::Timeout,

              // Overload — do NOT retry immediately, trips breaker
              SecurityError::CircuitOpen
              | SecurityError::ValidationOverloaded { .. } => ErrorClassification::Overload,

              // Degraded — partial success, does NOT trip breaker
              SecurityError::Degraded { .. }
              | SecurityError::FallbackUsed { .. } => ErrorClassification::Degraded,
          }
      }

      fn retry_hint(&self) -> RetryHint {
          match self {
              // Permanent errors — no retry
              SecurityError::LicenseInvalid(_)
              | SecurityError::LicenseRevoked(_)
              | SecurityError::HardwareMismatch(_)
              | SecurityError::BinaryTampered(_)
              | SecurityError::DebuggerDetected
              | SecurityError::AnalysisToolDetected(_) => RetryHint::no_retry(),

              // Transient errors — exponential backoff
              SecurityError::ServerValidationFailed { .. }
              | SecurityError::RevocationCheckFailed { .. }
              | SecurityError::HardwareFingerprintFailed { .. } => {
                  RetryHint::exponential(Duration::from_millis(500))
              }

              // Timeout errors — longer exponential backoff
              SecurityError::ServerTimeout { .. }
              | SecurityError::HeartbeatTimeout { .. } => {
                  RetryHint::exponential(Duration::from_secs(1))
              }

              // Overload errors — long backoff
              SecurityError::CircuitOpen
              | SecurityError::ValidationOverloaded { .. } => {
                  RetryHint::exponential(Duration::from_secs(2))
              }

              // Degraded errors — immediate retry for full result
              SecurityError::Degraded { .. }
              | SecurityError::FallbackUsed { .. } => RetryHint::immediate(),
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
      Ingestion(#[from] IngestionError),
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
      Middleware(#[from] MiddlewareError),
      #[error(transparent)]
      Metrics(#[from] MetricsError),
      #[error(transparent)]
      Anyhow(#[from] anyhow::Error),
      #[error("internal error: {0}")]
      Internal(String),
  }

  impl ErrorClassifier for AppError {
      fn classify(&self) -> ErrorClassification {
          match self {
              AppError::Redis(e) => e.classify(),
              AppError::Postgres(e) => e.classify(),
              AppError::ClickHouse(e) => e.classify(),
              AppError::Ingestion(e) => e.classify(),
              AppError::Cache(e) => e.classify(),
              AppError::Pipeline(e) => e.classify(),
              AppError::Model(e) => e.classify(),
              AppError::Scenario(e) => e.classify(),
              AppError::Security(e) => e.classify(),
              AppError::Middleware(e) => e.classify(),
              AppError::Metrics(e) => e.classify(),
              AppError::Anyhow(_) => ErrorClassification::Transient,
              AppError::Internal(_) => ErrorClassification::Transient,
          }
      }
  }

  pub type AppResult<T> = Result<T, AppError>;

  // ─── IntoResponse Implementation for AppError ───────────────────────────────

  use axum::{
      http::StatusCode,
      response::{IntoResponse, Response},
      Json,
  };
  use serde_json::json;
  use tracing::error;
  use chrono::Utc;

  impl IntoResponse for AppError {
      fn into_response(self) -> Response {
          let classification = self.classify();
          let (status, error_code, message) = match classification {
              ErrorClassification::Permanent => {
                  match &self {
                      AppError::Scenario(ScenarioError::NotFound(slug)) => {
                          (StatusCode::NOT_FOUND, "SCENARIO_NOT_FOUND", format!("Scenario '{}' not found", slug))
                      }
                      _ => (StatusCode::BAD_REQUEST, "PERMANENT_ERROR", "Request cannot be processed due to client error".to_string())
                  }
              }
              ErrorClassification::Transient => {
                  (StatusCode::BAD_GATEWAY, "TRANSIENT_ERROR", "Service temporarily unavailable".to_string())
              }
              ErrorClassification::Timeout => {
                  (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT_ERROR", "Request timed out".to_string())
              }
              ErrorClassification::Overload => {
                  (StatusCode::TOO_MANY_REQUESTS, "OVERLOAD_ERROR", "Service is overloaded".to_string())
              }
              ErrorClassification::Degraded => {
                  (StatusCode::MULTI_STATUS, "DEGRADED_ERROR", "Service returned degraded result".to_string())
              }
              ErrorClassification::PartialFailure => {
                  (StatusCode::MULTI_STATUS, "PARTIAL_FAILURE", "Partial failure occurred".to_string())
              }
          };

          error!(
              error = ?self,
              classification = ?classification,
              status_code = %status,
              error_code = error_code,
              "AppError converted to HTTP response"
          );

          let retry_hint = self.retry_hint();
          let retry_after = retry_hint.retry_after.map(|d| d.as_secs());

          let body = Json(json!({
              "success": false,
              "error": {
                  "message": message,
                  "code": error_code,
                  "classification": format!("{:?}", classification),
                  "retriable": classification.is_retriable(),
                  "retry_after": retry_after,
              },
              "status_code": status.as_u16(),
              "timestamp": Utc::now().to_rfc3339(),
          }));

          (status, body).into_response()
      }
  }
