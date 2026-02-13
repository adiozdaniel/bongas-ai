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

/// HTTP metrics middleware that integrates with AnalyticsManager
pub struct HttpMetricsMiddleware;

impl HttpMetricsMiddleware {
    pub fn new() -> Self {
        Self
    }

    pub async fn layer(
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        
        // Get analytics reference before borrowing req

        // Determine scenario from path

        // Record request

        let response = next.run(req).await;

        // Record success or failure

        response
    }

    fn _extract_scenario_from_path(path: &str) -> String {
        // Extract scenario from path patterns like /api/v1/recommendations/home/:user_id
        if path.contains("/home/") {
            "home".to_string()
        } else if path.contains("/continue-watching/") {
            "continue_watching".to_string()
        } else if path.contains("/trending") {
            "trending".to_string()
        } else if path.contains("/because-you-watched/") {
            "because_you_watched".to_string()
        } else if path.contains("/genre/") {
            "genre".to_string()
        } else if path.contains("/new-releases/") {
            "new_releases".to_string()
        } else if path.contains("/live-tv/") {
            "live_tv".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn _determine_user_type(req: &Request<Body>) -> String {
        // Try to extract user type from headers or default to "anonymous"
        req.headers()
            .get("X-User-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("anonymous")
            .to_string()
    }

    fn _classify_error(status: axum::http::StatusCode) -> String {
        if status.is_client_error() {
            format!("client_error_{}", status.as_u16())
        } else if status.is_server_error() {
            format!("server_error_{}", status.as_u16())
        } else {
            "unknown_error".to_string()
        }
    }
}

/// Endpoint-specific metrics tracking
pub struct EndpointMetrics {
    endpoint_stats: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, EndpointStats>>>,
}

#[derive(Debug, Clone)]
struct EndpointStats {
    total_requests: u64,
    success_requests: u64,
    error_requests: u64,
    total_latency_ms: u64,
    last_accessed: chrono::DateTime<chrono::Utc>,
}

impl EndpointMetrics {
    pub fn new() -> Self {
        Self {
            endpoint_stats: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn layer(
        self: Arc<Self>,
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        let path = req.uri().path().to_string();
        let _method = req.method().clone();

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
}
