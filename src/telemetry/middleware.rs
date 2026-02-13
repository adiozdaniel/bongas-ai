  //! HTTP tracing middleware for request/response logging.
  //!
  //! Provides Axum-compatible middleware for automatic request tracing
  //! with correlation IDs and latency measurement.

  use axum::{
      body::Body,
      extract::Request,
      middleware::Next,
      response::Response,
  };
  use std::time::Instant;
  use tracing::{info, warn, error, Span};

  use super::context::{headers, RequestId, TraceContext};

  // ─── Tracing Middleware ─────────────────────────────────────────────────────

  /// HTTP tracing middleware with correlation IDs and structured logging.
  ///
  /// Automatically:
  /// - Extracts or generates request IDs
  /// - Creates spans for each request
  /// - Records latency and status codes
  /// - Propagates correlation headers in responses
  pub async fn tracing_middleware(
      mut req: Request<Body>,
      next: Next,
  ) -> Response<Body> {
      // Extract or generate request ID
      let request_id = extract_or_generate_request_id(&req);

      // Extract optional context from headers
      let user_id = extract_header(&req, headers::USER_ID);
      let session_id = extract_header(&req, headers::SESSION_ID);

      // Build trace context (for future use with distributed tracing)
      let _ctx = {
          let mut ctx = TraceContext::with_request_id(request_id.clone());
          if let Some(ref uid) = user_id {
              ctx = ctx.with_user_id(uid.clone());
          }
          if let Some(ref sid) = session_id {
              ctx = ctx.with_session_id(sid.clone());
          }
          ctx
      };

      // Extract request metadata
      let method = req.method().clone();
      let uri = req.uri().clone();
      let path = uri.path().to_string();
      let version = format!("{:?}", req.version());
      let user_agent = extract_header(&req, "user-agent").unwrap_or_default();

      // Add request ID to request headers (for downstream services)
      if let Ok(header_value) = request_id.as_str().parse() {
          req.headers_mut().insert(headers::REQUEST_ID, header_value);
      }

      // Create span
      let span = tracing::info_span!(
          "http_request",
          request_id = %request_id,
          method = %method,
          path = %path,
          version = %version,
          user_agent = %user_agent,
          user_id = tracing::field::Empty,
          session_id = tracing::field::Empty,
          status_code = tracing::field::Empty,
          latency_ms = tracing::field::Empty,
      );

      // Record optional user context
      if let Some(ref uid) = user_id {
          span.record("user_id", uid.as_str());
      }
      if let Some(ref sid) = session_id {
          span.record("session_id", sid.as_str());
      }

      // Execute request within span
      let start = Instant::now();
      let response = {
          let _guard = span.enter();

          info!(
              request_id = %request_id,
              method = %method,
              path = %path,
              "request started"
          );

          next.run(req).await
      };
      let latency = start.elapsed();
      let latency_ms = latency.as_millis() as u64;

      // Extract response status
      let status = response.status();
      let status_code = status.as_u16();

      // Record status and latency
      span.record("status_code", status_code);
      span.record("latency_ms", latency_ms);

      // Log based on status
      let _guard = span.enter();
      if status.is_success() {
          info!(
              request_id = %request_id,
              status = %status_code,
              latency_ms = %latency_ms,
              "request completed"
          );
      } else if status.is_client_error() {
          warn!(
              request_id = %request_id,
              status = %status_code,
              latency_ms = %latency_ms,
              "request completed with client error"
          );
      } else {
          error!(
              request_id = %request_id,
              status = %status_code,
              latency_ms = %latency_ms,
              "request completed with server error"
          );
      }

      // Add correlation headers to response
      let mut response = response;
      if let Ok(header_value) = request_id.as_str().parse() {
          response.headers_mut().insert(headers::REQUEST_ID, header_value);
      }

      response
  }

  // ─── Helper Functions ───────────────────────────────────────────────────────

  /// Extract request ID from headers or generate a new one.
  fn extract_or_generate_request_id(req: &Request<Body>) -> RequestId {
      req.headers()
          .get(headers::REQUEST_ID)
          .and_then(|h| h.to_str().ok())
          .map(RequestId::from_string)
          .unwrap_or_else(RequestId::generate)
  }

  /// Extract a header value as String.
  fn extract_header(req: &Request<Body>, name: &str) -> Option<String> {
      req.headers()
          .get(name)
          .and_then(|h| h.to_str().ok())
          .map(String::from)
  }

  // ─── Span Instrumentation Helpers ───────────────────────────────────────────

  /// Create a database operation span.
  #[inline]
  pub fn db_span(operation: &'static str, table: &str) -> Span {
      tracing::info_span!(
          "db",
          otel.kind = "client",
          db.system = "postgresql",
          db.operation = operation,
          db.sql.table = table,
      )
  }

  /// Create a Redis operation span.
  #[inline]
  pub fn redis_span(operation: &'static str, key: &str) -> Span {
      tracing::info_span!(
          "redis",
          otel.kind = "client",
          db.system = "redis",
          db.operation = operation,
          db.redis.key = key,
      )
  }

  /// Create a Kafka operation span.
  #[inline]
  pub fn kafka_span(operation: &'static str, topic: &str) -> Span {
      tracing::info_span!(
          "kafka",
          otel.kind = if operation == "send" { "producer" } else { "consumer" },
          messaging.system = "kafka",
          messaging.operation = operation,
          messaging.destination.name = topic,
      )
  }

  /// Create an ML inference span.
  #[inline]
  pub fn ml_span(model: &str, operation: &'static str) -> Span {
      tracing::info_span!(
          "ml_inference",
          otel.kind = "internal",
          ml.model = model,
          ml.operation = operation,
      )
  }
