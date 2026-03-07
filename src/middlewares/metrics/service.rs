use axum::{
      extract::{Request, Extension},
      middleware::Next,
      response::Response,
      body::Body,
  };
  use std::sync::Arc;
  use tokio::sync::Mutex;
  use std::time::Instant;
  use chrono::Utc;

  /// Metrics collector for tracking API performance and usage
  pub struct MetricsCollector {
      // Fix #36: Replace std::sync::Mutex with tokio::sync::Mutex in async context
      scenario_stats: Arc<Mutex<std::collections::HashMap<String, ScenarioStats>>>,
  }

  #[derive(Debug, Clone, Default)]
  pub struct ScenarioStats {
      pub total_executions: u64,
      pub cache_hits: u64,
      pub total_latency_ms: u64,
  }

  impl MetricsCollector {
      pub fn new() -> Self {
          Self {
              scenario_stats: Arc::new(Mutex::new(std::collections::HashMap::new())),
          }
      }

      pub async fn record_scenario_execution(&self, scenario: &str, latency_ms: u64, cache_hit: bool) {
          let mut stats_map = self.scenario_stats.lock().await;
          let stats = stats_map.entry(scenario.to_string()).or_default();
          stats.total_executions += 1;
          if cache_hit {
              stats.cache_hits += 1;
          }
          stats.total_latency_ms += latency_ms;
      }

      pub async fn get_scenario_stats(&self) -> Vec<(String, ScenarioStats)> {
          let stats_map = self.scenario_stats.lock().await;
          stats_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
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
      // Fix #36: Replace std::sync::Mutex with tokio::sync::Mutex in async context
      endpoint_stats: Arc<Mutex<std::collections::HashMap<String, EndpointStats>>>,
  }

  #[derive(Debug, Clone)]
  pub struct EndpointStats {
      pub total_requests: u64,
      pub success_requests: u64,
      pub error_requests: u64,
      pub total_latency_ms: u64,
      pub last_accessed: chrono::DateTime<chrono::Utc>,
  }

  impl EndpointMetrics {
      pub fn new() -> Self {
          Self {
              endpoint_stats: Arc::new(Mutex::new(std::collections::HashMap::new())),
          }
      }

      pub async fn layer(
          Extension(state): Extension<Arc<Self>>,
          req: Request<Body>,
          next: Next,
      ) -> Response {
          let path = req.uri().path().to_string();

          let start = Instant::now();
          let response = next.run(req).await;
          let duration = start.elapsed();

          let status = response.status();
          let duration_ms = duration.as_millis() as u64;

          // Update endpoint stats
          {
              let mut stats_map = state.endpoint_stats.lock().await;
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

      pub async fn get_stats(&self) -> Vec<(String, EndpointStats)> {
          let stats_map = self.endpoint_stats.lock().await;
          stats_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
      }
  }
