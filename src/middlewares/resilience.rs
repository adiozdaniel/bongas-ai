  //! Netflix-grade resilience middleware using circuit breaker pattern.
  //!
  //! Wraps HTTP endpoints with circuit breakers to prevent cascading failures.
  //! Integrates with existing CircuitBreakerRegistry which already has
  //! ResilienceMetricsCollector wired as an observer.
  //!
  //! # Pattern Integration
  //! - **Observer**: CircuitBreaker automatically emits events to ResilienceMetricsCollector
  //! - **Strategy**: HttpError implements ErrorClassifier for classification
  //! - **Composite**: Metrics aggregated via MetricsRegistry
  //! - **Registry**: CircuitBreakerRegistry manages all breakers

  use axum::{
      body::Body,
      extract::{Request, State},
      http::StatusCode,
      middleware::Next,
      response::{IntoResponse, Response},
  };
  use serde_json::json;
  use std::sync::Arc;
  use tracing::{error, warn};

  use crate::circuit_breaker::{
      CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerId,
      CircuitBreakerRegistry, CircuitState,
  };
  use crate::error::{ErrorClassification, ErrorClassifier};

  /// Resilience middleware state shared across requests.
  #[derive(Clone)]
  pub struct ResilienceMiddleware {
      registry: Arc<CircuitBreakerRegistry>,
      config: ResilienceConfig,
  }

  /// Configuration for resilience middleware.
  #[derive(Clone)]
  pub struct ResilienceConfig {
      /// Whether to enable circuit breakers for all endpoints
      pub enabled: bool,
      /// Default circuit breaker config for endpoints
      pub default_breaker_config: CircuitBreakerConfig,
  }

  impl Default for ResilienceConfig {
      fn default() -> Self {
          Self {
              enabled: true,
              default_breaker_config: CircuitBreakerConfig::default(),
          }
      }
  }

  impl ResilienceMiddleware {
      /// Create a new resilience middleware.
      ///
      /// The registry should already be created with ResilienceMetricsCollector as observer.
      pub fn new(registry: Arc<CircuitBreakerRegistry>) -> Self {
          Self {
              registry,
              config: ResilienceConfig::default(),
          }
      }

      /// Create with custom configuration.
      pub fn with_config(
          registry: Arc<CircuitBreakerRegistry>,
          config: ResilienceConfig,
      ) -> Self {
          Self {
              registry,
              config,
          }
      }

      /// Middleware layer that wraps requests with circuit breaker protection.
      ///
      /// All metrics recording is automatic via the Observer pattern - the CircuitBreaker
      /// emits events that are captured by ResilienceMetricsCollector.
      pub async fn layer(
          State(middleware): State<Arc<Self>>,
          req: Request,
          next: Next,
      ) -> Response {
          if !middleware.config.enabled {
              return next.run(req).await;
          }

          let endpoint = extract_endpoint(&req);
          // Leak string to get 'static lifetime for CircuitBreakerId
          let endpoint_static: &'static str = Box::leak(endpoint.into_boxed_str());
          let breaker_id = CircuitBreakerId::new(endpoint_static);

          // Get or create circuit breaker for this endpoint
          let breaker = middleware.registry.get_or_create(
              breaker_id.clone(),
              middleware.config.default_breaker_config.clone(),
          );

          // Execute request through circuit breaker
          // The breaker automatically records all metrics via observer pattern
          let result = breaker
              .call(|| async {
                  let response = next.run(req).await;
                  classify_response(response).await
              })
              .await;

          match result {
              Ok(response) => {
                  // Success automatically recorded via observer
                  response
              }
              Err(CircuitBreakerError::Rejected { state, retry_after }) => {
                  warn!(
                      breaker_id = %breaker_id.label(),
                      state = ?state,
                      retry_after = ?retry_after,
                      "Request rejected by circuit breaker"
                  );

                  // Rejection automatically recorded via observer
                  create_rejection_response(state, retry_after)
              }
              Err(CircuitBreakerError::ExecutionFailed {
                  source,
                  classification,
                  latency,
              }) => {
                  error!(
                      breaker_id = %breaker_id.label(),
                      classification = ?classification,
                      latency_ms = latency.as_millis(),
                      "Request execution failed through circuit breaker"
                  );

                  // Failure automatically recorded via observer with classification
                  // Return the original response
                  source.response
              }
              Err(CircuitBreakerError::TimedOut { timeout }) => {
                  error!(
                      breaker_id = %breaker_id.label(),
                      timeout_ms = timeout.as_millis(),
                      "Request timed out"
                  );

                  // Timeout automatically recorded via observer
                  create_timeout_response(timeout)
              }
          }
      }
  }

  /// Extract endpoint identifier from request.
  fn extract_endpoint(req: &Request) -> String {
      let path = req.uri().path();
      let method = req.method();

      // Normalize path by replacing IDs with placeholders
      let normalized_path = normalize_path(path);

      format!("http_{}_{}", method.as_str().to_lowercase(), normalized_path)
  }

  /// Normalize path by replacing dynamic segments with placeholders.
  fn normalize_path(path: &str) -> String {
      path.split('/')
          .map(|segment| {
              // Replace UUIDs and numeric IDs with placeholders
              if segment.parse::<i64>().is_ok() {
                  ":id"
              } else if segment.len() == 36 && segment.chars().filter(|c| *c == '-').count() == 4 {
                  ":uuid"
              } else {
                  segment
              }
          })
          .collect::<Vec<_>>()
          .join("/")
  }

  /// Classify HTTP response for circuit breaker logic.
  async fn classify_response(response: Response) -> Result<Response, HttpError> {
      let status = response.status();

      if status.is_success() || status.is_redirection() {
          Ok(response)
      } else {
          Err(HttpError {
              status,
              response,
          })
      }
  }

  /// HTTP error wrapper for circuit breaker integration.
  ///
  /// Implements ErrorClassifier (Strategy pattern) so the circuit breaker
  /// can properly classify HTTP errors.
  #[derive(Debug)]
  struct HttpError {
      status: StatusCode,
      response: Response,
  }

  impl std::fmt::Display for HttpError {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "HTTP error: {}", self.status)
      }
  }

  impl std::error::Error for HttpError {}

  impl ErrorClassifier for HttpError {
      fn classify(&self) -> ErrorClassification {
          match self.status.as_u16() {
              // 5xx errors are transient (server issues)
              500..=599 => {
                  if self.status == StatusCode::SERVICE_UNAVAILABLE {
                      ErrorClassification::Overload
                  } else if self.status == StatusCode::GATEWAY_TIMEOUT {
                      ErrorClassification::Timeout
                  } else {
                      ErrorClassification::Transient
                  }
              }
              // 429 is overload
              429 => ErrorClassification::Overload,
              // 408 is timeout
              408 => ErrorClassification::Timeout,
              // 4xx errors are permanent (client issues)
              400..=499 => ErrorClassification::Permanent,
              // Everything else is transient
              _ => ErrorClassification::Transient,
          }
      }
  }

  /// Create rejection response when circuit breaker is open.
  fn create_rejection_response(state: CircuitState, retry_after: Option<std::time::Duration>) -> Response {
      let retry_after_secs = retry_after.map(|d| d.as_secs()).unwrap_or(60);

      let body = json!({
          "success": false,
          "error": "Service temporarily unavailable",
          "code": "CIRCUIT_BREAKER_OPEN",
          "message": format!("Circuit breaker is {:?}. Please try again later.", state),
          "retry_after_seconds": retry_after_secs,
          "timestamp": chrono::Utc::now().to_rfc3339(),
      });

      let mut response = (
          StatusCode::SERVICE_UNAVAILABLE,
          Body::from(body.to_string()),
      )
      .into_response();

      // Add Retry-After header
      response.headers_mut().insert(
          "Retry-After",
          retry_after_secs.to_string().parse().unwrap(),
      );

      response
  }

  /// Create timeout response.
  fn create_timeout_response(timeout: std::time::Duration) -> Response {
      let body = json!({
          "success": false,
          "error": "Request timeout",
          "code": "GATEWAY_TIMEOUT",
          "message": format!("Request timed out after {}ms", timeout.as_millis()),
          "timestamp": chrono::Utc::now().to_rfc3339(),
      });

      (
          StatusCode::GATEWAY_TIMEOUT,
          Body::from(body.to_string()),
      )
      .into_response()
  }

