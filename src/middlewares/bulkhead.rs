
  //! Bulkhead middleware for request concurrency limiting.
  //!
  //! Implements Netflix-style bulkhead pattern to prevent resource exhaustion
  //! by limiting concurrent requests per endpoint.

  use axum::{
      body::Body,
      extract::{Request, State},
      http::StatusCode,
      middleware::Next,
      response::{IntoResponse, Response},
  };
  use serde_json::json;
  use dashmap::DashMap;
  use std::sync::Arc;
  use tokio::sync::Semaphore;
  use tracing::warn;

  /// Bulkhead middleware state.
  #[derive(Clone)]
  pub struct BulkheadMiddleware {
      semaphores: Arc<DashMap<String, Arc<Semaphore>>>,
      config: BulkheadConfig,
  }

  /// Bulkhead configuration.
  #[derive(Clone)]
  pub struct BulkheadConfig {
      /// Maximum concurrent requests per endpoint
      pub max_concurrent_calls: usize,
      /// Whether to enable bulkhead
      pub enabled: bool,
      /// Whether to use per-endpoint limits (vs global)
      pub per_endpoint: bool,
      /// Maximum number of unique endpoint bulkheads to track before falling back
      /// to global to prevent memory exhaustion.
      pub max_tracked_endpoints: usize,
  }

  impl Default for BulkheadConfig {
      fn default() -> Self {
          Self {
              max_concurrent_calls: 100,
              enabled: true,
              per_endpoint: true,
              max_tracked_endpoints: 1000,
          }
      }
  }

  impl BulkheadMiddleware {
      /// Create a new bulkhead middleware.
      pub fn new(config: BulkheadConfig) -> Self {
          Self {
              semaphores: Arc::new(DashMap::new()),
              config,
          }
      }

      /// Create with default configuration.
      pub fn with_defaults() -> Self {
          Self::new(BulkheadConfig::default())
      }

      /// Middleware layer that limits concurrent requests.
      pub async fn layer(
          State(state): State<Arc<Self>>,
          req: Request,
          next: Next,
      ) -> Response {
          if !state.config.enabled {
              return next.run(req).await;
          }

          let endpoint = if state.config.per_endpoint {
              extract_endpoint(&req)
          } else {
              "global".to_string()
          };

          // Get or create semaphore for this endpoint
          let semaphore = state.get_or_create_semaphore(&endpoint);

          // Try to acquire permit
          let permit = match semaphore.try_acquire_owned() {
              Ok(permit) => permit,
              Err(_) => {
                  warn!(
                      endpoint = %endpoint,
                      max_concurrent = state.config.max_concurrent_calls,
                      "Bulkhead limit exceeded"
                  );
                  return create_bulkhead_rejection_response();
              }
          };

          // Execute request with permit held
          let response = next.run(req).await;

          // Permit is automatically released when dropped
          drop(permit);

          response
      }

      /// Get or create semaphore for an endpoint with protection against unbounded growth.
      fn get_or_create_semaphore(&self, endpoint: &str) -> Arc<Semaphore> {
          if let Some(sem) = self.semaphores.get(endpoint) {
              return sem.clone();
          }

          // Protection against cardinality explosion (DoS via unique paths)
          if self.semaphores.len() >= self.config.max_tracked_endpoints {
              warn!(
                  endpoint = %endpoint,
                  "Bulkhead tracking limit reached. Falling back to global limit to prevent memory exhaustion."
              );
              return self.semaphores.entry("global".to_string())
                  .or_insert_with(|| Arc::new(Semaphore::new(self.config.max_concurrent_calls)))
                  .clone();
          }

          self.semaphores.entry(endpoint.to_string())
              .or_insert_with(|| Arc::new(Semaphore::new(self.config.max_concurrent_calls)))
              .clone()
      }
  }

  /// Extract endpoint identifier from request.
  fn extract_endpoint(req: &Request) -> String {
      let path = req.uri().path();
      let method = req.method();
      format!("{}_{}", method.as_str().to_lowercase(), normalize_path(path))
  }

  /// Normalize path by replacing dynamic segments.
  fn normalize_path(path: &str) -> String {
      path.split('/')
          .map(|segment| {
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

  /// Create rejection response when bulkhead is full.
  fn create_bulkhead_rejection_response() -> Response {
      let body = json!({
          "success": false,
          "error": "Too many concurrent requests",
          "code": "BULKHEAD_FULL",
          "message": "The service is currently handling too many concurrent requests. Please try again later.",
          "retry_after_seconds": 1,
          "timestamp": chrono::Utc::now().to_rfc3339(),
      });

      let mut response = (
          StatusCode::SERVICE_UNAVAILABLE,
          Body::from(body.to_string()),
      )
      .into_response();

      response.headers_mut().insert(
          "Retry-After",
          "1".parse().unwrap(),
      );

      response
  }
