
  //! Span context and correlation IDs for distributed tracing.
  //!
  //! Provides utilities for propagating trace context across service boundaries
  //! and maintaining correlation IDs throughout request lifecycles.

  use std::sync::atomic::{AtomicU64, Ordering};
  use tracing::Span;

  // ─── Request ID Generation ──────────────────────────────────────────────────

  /// Atomic counter for generating unique request IDs within this process.
  static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

  /// Unique identifier for a request.
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  pub struct RequestId(String);

  impl RequestId {
      /// Generate a new unique request ID.
      ///
      /// Format: `{timestamp_ms}-{process_counter}-{random}`
      pub fn generate() -> Self {
          let timestamp = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)
              .map(|d| d.as_millis())
              .unwrap_or(0);

          let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
          let random: u32 = rand_simple();

          Self(format!("{:x}-{:04x}-{:04x}", timestamp, counter, random))
      }

      /// Create from an existing string (e.g., from incoming header).
      pub fn from_string(s: impl Into<String>) -> Self {
          Self(s.into())
      }

      /// Get the string representation.
      pub fn as_str(&self) -> &str {
          &self.0
      }
  }

  impl std::fmt::Display for RequestId {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "{}", self.0)
      }
  }

  impl Default for RequestId {
      fn default() -> Self {
          Self::generate()
      }
  }

  /// Simple pseudo-random number generator (no external dependency).
  fn rand_simple() -> u32 {
      use std::collections::hash_map::RandomState;
      use std::hash::{BuildHasher, Hasher};

      let state = RandomState::new();
      let mut hasher = state.build_hasher();
      hasher.write_u64(REQUEST_COUNTER.load(Ordering::Relaxed));
      hasher.finish() as u32
  }

  // ─── Trace Context ──────────────────────────────────────────────────────────

  /// Trace context for distributed tracing.
  ///
  /// Contains all information needed to correlate logs and traces
  /// across service boundaries.
  #[derive(Debug, Clone)]
  pub struct TraceContext {
      /// Unique request identifier.
      pub request_id: RequestId,
      /// Parent span ID (if any).
      pub parent_span_id: Option<String>,
      /// User ID (if authenticated).
      pub user_id: Option<String>,
      /// Session ID (if applicable).
      pub session_id: Option<String>,
      /// Additional baggage items.
      pub baggage: Vec<(String, String)>,
  }

  impl TraceContext {
      /// Create a new trace context with a generated request ID.
      pub fn new() -> Self {
          Self {
              request_id: RequestId::generate(),
              parent_span_id: None,
              user_id: None,
              session_id: None,
              baggage: Vec::new(),
          }
      }

      /// Create from an existing request ID.
      pub fn with_request_id(request_id: RequestId) -> Self {
          Self {
              request_id,
              parent_span_id: None,
              user_id: None,
              session_id: None,
              baggage: Vec::new(),
          }
      }

      /// Set the user ID.
      pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
          self.user_id = Some(user_id.into());
          self
      }

      /// Set the session ID.
      pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
          self.session_id = Some(session_id.into());
          self
      }

      /// Add a baggage item.
      pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
          self.baggage.push((key.into(), value.into()));
          self
      }

      /// Create a tracing span with this context.
      pub fn create_span(&self, name: &'static str) -> Span {
          let span = tracing::info_span!(
              "request",
              otel.name = name,
              request_id = %self.request_id,
          );

          if let Some(ref user_id) = self.user_id {
              span.record("user_id", user_id.as_str());
          }

          if let Some(ref session_id) = self.session_id {
              span.record("session_id", session_id.as_str());
          }

          span
      }
  }

  impl Default for TraceContext {
      fn default() -> Self {
          Self::new()
      }
  }

  // ─── Span Extensions ────────────────────────────────────────────────────────

  /// Extension trait for adding context to spans.
  pub trait SpanExt {
      /// Record the request ID on this span.
      fn record_request_id(&self, request_id: &RequestId);

      /// Record the user ID on this span.
      fn record_user_id(&self, user_id: &str);

      /// Record an error on this span.
      fn record_error(&self, error: &dyn std::error::Error);
  }

  impl SpanExt for Span {
      fn record_request_id(&self, request_id: &RequestId) {
          self.record("request_id", request_id.as_str());
      }

      fn record_user_id(&self, user_id: &str) {
          self.record("user_id", user_id);
      }

      fn record_error(&self, error: &dyn std::error::Error) {
          self.record("error", true);
          self.record("error.message", error.to_string().as_str());

          if let Some(source) = error.source() {
              self.record("error.source", source.to_string().as_str());
          }
      }
  }

  // ─── Header Constants ───────────────────────────────────────────────────────

  /// HTTP header names for trace context propagation.
  pub mod headers {
      /// Request ID header.
      pub const REQUEST_ID: &str = "X-Request-ID";
      /// Trace ID header (W3C Trace Context).
      pub const TRACE_ID: &str = "traceparent";
      /// Baggage header (W3C Baggage).
      pub const BAGGAGE: &str = "baggage";
      /// User ID header.
      pub const USER_ID: &str = "X-User-ID";
      /// Session ID header.
      pub const SESSION_ID: &str = "X-Session-ID";
  }

