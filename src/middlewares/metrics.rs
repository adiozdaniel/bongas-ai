use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    body::Body,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use serde_json::json;
use chrono::Utc;

/// Metrics collector for tracking API performance and usage
pub struct MetricsCollector {
    total_requests: Arc<AtomicU64>,
    success_requests: Arc<AtomicU64>,
    error_requests: Arc<AtomicU64>,
    client_error_requests: Arc<AtomicU64>,
    server_error_requests: Arc<AtomicU64>,
    total_latency_ms: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            success_requests: Arc::new(AtomicU64::new(0)),
            error_requests: Arc::new(AtomicU64::new(0)),
            client_error_requests: Arc::new(AtomicU64::new(0)),
            server_error_requests: Arc::new(AtomicU64::new(0)),
            total_latency_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn layer(
        self: Arc<Self>,
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let start_time = Instant::now();
        let response = next.run(req).await;
        let latency = start_time.elapsed();

        let status = response.status();
        let latency_ms = latency.as_millis() as u64;

        // Update counters based on status
        if status.is_success() {
            self.success_requests.fetch_add(1, Ordering::Relaxed);
        } else if status.is_client_error() {
            self.error_requests.fetch_add(1, Ordering::Relaxed);
            self.client_error_requests.fetch_add(1, Ordering::Relaxed);
        } else if status.is_server_error() {
            self.error_requests.fetch_add(1, Ordering::Relaxed);
            self.server_error_requests.fetch_add(1, Ordering::Relaxed);
        }

        // Update total latency
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);

        // Add metrics headers to response
        let mut response = response;
        response.headers_mut().insert(
            "X-Request-Count",
            self.total_requests.load(Ordering::Relaxed).to_string().parse().unwrap(),
        );

        response.headers_mut().insert(
            "X-Response-Time",
            latency_ms.to_string().parse().unwrap(),
        );

        response
    }

    /// Get current metrics statistics
    pub fn get_stats(&self) -> MetricsStats {
        let total = self.total_requests.load(Ordering::Relaxed);
        let success = self.success_requests.load(Ordering::Relaxed);
        let errors = self.error_requests.load(Ordering::Relaxed);
        let client_errors = self.client_error_requests.load(Ordering::Relaxed);
        let server_errors = self.server_error_requests.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);

        let avg_latency = if total > 0 {
            total_latency / total
        } else {
            0
        };

        let success_rate = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        MetricsStats {
            total_requests: total,
            success_requests: success,
            error_requests: errors,
            client_error_requests: client_errors,
            server_error_requests: server_errors,
            average_latency_ms: avg_latency,
            success_rate,
            timestamp: Utc::now(),
        }
    }

    /// Get metrics as JSON for API endpoints
    pub fn get_stats_json(&self) -> serde_json::Value {
        let stats = self.get_stats();
        json!({
            "total_requests": stats.total_requests,
            "success_requests": stats.success_requests,
            "error_requests": stats.error_requests,
            "client_error_requests": stats.client_error_requests,
            "server_error_requests": stats.server_error_requests,
            "average_latency_ms": stats.average_latency_ms,
            "success_rate": stats.success_rate,
            "timestamp": stats.timestamp.to_rfc3339(),
        })
    }

    /// Reset all metrics counters
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.success_requests.store(0, Ordering::Relaxed);
        self.error_requests.store(0, Ordering::Relaxed);
        self.client_error_requests.store(0, Ordering::Relaxed);
        self.server_error_requests.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct MetricsStats {
    pub total_requests: u64,
    pub success_requests: u64,
    pub error_requests: u64,
    pub client_error_requests: u64,
    pub server_error_requests: u64,
    pub average_latency_ms: u64,
    pub success_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
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
        let method = req.method().clone();

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

    pub fn get_endpoint_stats(&self) -> serde_json::Value {
        let stats_map = self.endpoint_stats.lock().unwrap();
        let mut result = std::collections::HashMap::new();

        for (path, stats) in stats_map.iter() {
            let avg_latency = if stats.total_requests > 0 {
                stats.total_latency_ms / stats.total_requests
            } else {
                0
            };

            let success_rate = if stats.total_requests > 0 {
                (stats.success_requests as f64 / stats.total_requests as f64) * 100.0
            } else {
                0.0
            };

            result.insert(path.clone(), json!({
                "total_requests": stats.total_requests,
                "success_requests": stats.success_requests,
                "error_requests": stats.error_requests,
                "average_latency_ms": avg_latency,
                "success_rate": success_rate,
                "last_accessed": stats.last_accessed.to_rfc3339(),
            }));
        }

        json!(result)
    }
}