  use axum::{
      extract::Request,
      middleware::Next,
      response::Response,
      body::Body,
  };
  use std::sync::Arc;
  use std::time::Instant;
  use chrono::Utc;

  /// Metrics collector for tracking API performance and usage
  pub struct MetricsCollector;

  impl MetricsCollector {
      pub fn new() -> Self {
          Self
      }
  }

  /// Request duration tracking middleware
  pub struct DurationTracker;

  impl DurationTracker {
      pub async fn layer(
          req: Request<Body>,
          next: Next,
      ) -> Response<Body> {
          let start = Instant::now();
          let method = req.method().clone();
          let uri = req.uri().clone();

          let response = next.run(req).await;
          let duration = start.elapsed();

          let status = response.status();
          let duration_ms = duration.as_millis();

          // Log slow requests
          if duration_ms > 1000 {
              tracing::warn!(
                  method = %method,
                  uri = %uri,
                  duration_ms = %duration_ms,
                  status = %status,
                  "Slow request detected"
              );
          }

          // Add duration header
          let mut response = response;
          response.headers_mut().insert(
              "X-Request-Duration",
              format!("{}ms", duration_ms).parse().unwrap(),
          );

          response
      }
  }

  /// Endpoint-specific metrics tracking
  pub struct EndpointMetrics {
      endpoint_stats: Arc<std::sync::Mutex<std::collections::HashMap<String, EndpointStats>>>,
  }

  #[derive(Debug, Clone)]
  pub struct EndpointStats {
      total_requests: u64,
      success_requests: u64,
      error_requests: u64,
      total_latency_ms: u64,
      last_accessed: chrono::DateTime<chrono::Utc>,
  }

  impl EndpointMetrics {
      pub fn new() -> Self {
          Self {
              endpoint_stats: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
          }
      }

      pub async fn layer(
          self: Arc<Self>,
          req: Request<Body>,
          next: Next,
      ) -> Response<Body> {
          let path = req.uri().path().to_string();

          let start = Instant::now();
          let response = next.run(req).await;
          let duration = start.elapsed();

          let status = response.status();
          let duration_ms = duration.as_millis() as u64;

          // Update endpoint stats
          {
              let mut stats_map = self.endpoint_stats.lock().unwrap();
              let stats = stats_map.entry(path.clone()).or_insert_with(|| EndpointStats {
                  total_requests: 0,
                  success_requests: 0,
                  error_requests: 0,
                  total_latency_ms: 0,
                  last_accessed: Utc::now(),
              });

              stats.total_requests += 1;
              if status.is_success() {
                  stats.success_requests += 1;
              } else {
                  stats.error_requests += 1;
              }
              stats.total_latency_ms += duration_ms;
              stats.last_accessed = Utc::now();
          }

          response
      }

      pub fn get_stats(&self) -> Vec<(String, EndpointStats)> {
          let stats_map = self.endpoint_stats.lock().unwrap();
          stats_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
      }
  }

