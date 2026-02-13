//! HTTP middleware-specific instrumentation.
//!
//! Provides comprehensive Prometheus metrics for monitoring HTTP middleware components,
//! including request/response characteristics, performance, rate limiting, compression,
//! CORS handling, and error processing. Enables observability of the HTTP request
//! lifecycle and middleware chain behavior.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for HTTP middleware operations and request processing.
///
/// Maintains comprehensive metric vectors for:
/// * HTTP request throughput and characteristics
/// * Response status code distribution
/// * Middleware execution performance and reliability
/// * Rate limiting effectiveness and blocked requests
/// * Compression algorithm performance and efficiency
/// * CORS policy enforcement and cross-origin traffic
/// * Error response classification and handling latency
pub struct MiddlewareMetrics {
    /// Total HTTP requests, labeled by HTTP method and request path.
    pub http_requests: IntCounterVec,
    /// HTTP request duration distribution, labeled by method and path.
    pub http_request_duration: HistogramVec,
    /// HTTP response size in bytes, labeled by method and path.
    pub http_response_size: HistogramVec,
    /// HTTP request size in bytes, labeled by method and path.
    pub http_request_size: HistogramVec,

    /// Total HTTP status codes returned, labeled by status code.
    pub http_status_codes: IntCounterVec,

    /// Total middleware executions, labeled by middleware name.
    pub middleware_executions: IntCounterVec,
    /// Successful middleware executions, labeled by middleware name.
    pub middleware_successes: IntCounterVec,
    /// Failed middleware executions, labeled by middleware name.
    pub middleware_failures: IntCounterVec,
    /// Middleware execution duration distribution, labeled by middleware name.
    pub middleware_duration: HistogramVec,

    /// Rate limit hits (requests within limits), labeled by client IP.
    pub rate_limit_hits: IntCounterVec,
    /// Rate limit misses (exceeded limits), labeled by client IP.
    pub rate_limit_misses: IntCounterVec,
    /// Rate limit blocks (rejected requests), labeled by client IP.
    pub rate_limit_blocked: IntCounterVec,

    /// Total compression attempts, labeled by compression algorithm.
    pub compression_attempts: IntCounterVec,
    /// Successful compression operations, labeled by algorithm.
    pub compression_successes: IntCounterVec,
    /// Failed compression operations, labeled by algorithm.
    pub compression_failures: IntCounterVec,
    /// Compression ratio distribution (compressed/original), labeled by algorithm.
    pub compression_ratio: HistogramVec,

    /// Total CORS requests, labeled by origin and HTTP method.
    pub cors_requests: IntCounterVec,
    /// Allowed CORS requests, labeled by origin and method.
    pub cors_allowed: IntCounterVec,
    /// Blocked CORS requests, labeled by origin and method.
    pub cors_blocked: IntCounterVec,

    /// Total error responses, labeled by error type.
    pub error_responses: IntCounterVec,
    /// Total error types encountered, labeled by error type.
    pub error_types: IntCounterVec,
    /// Error handling duration distribution, labeled by error type.
    pub error_handling_duration: HistogramVec,
}

impl MiddlewareMetrics {
    /// Creates and registers all HTTP middleware metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // HTTP request metrics
            http_requests: register_int_counter_vec_with_registry!(
                opts!("http_requests_total", "Total HTTP requests"),
                &[labels::METHOD, labels::PATH],
                registry
            )?,
            http_request_duration: register_histogram_vec_with_registry!(
                format!("{}_http_request_duration_seconds", NAMESPACE),
                "HTTP request duration",
                &[labels::METHOD, labels::PATH],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            http_response_size: register_histogram_vec_with_registry!(
                format!("{}_http_response_size_bytes", NAMESPACE),
                "HTTP response size in bytes",
                &[labels::METHOD, labels::PATH],
                vec![0.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0],
                registry
            )?,
            http_request_size: register_histogram_vec_with_registry!(
                format!("{}_http_request_size_bytes", NAMESPACE),
                "HTTP request size in bytes",
                &[labels::METHOD, labels::PATH],
                vec![0.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0],
                registry
            )?,

            // Status code metrics
            http_status_codes: register_int_counter_vec_with_registry!(
                opts!("http_status_codes_total", "Total HTTP status codes"),
                &[labels::STATUS_CODE],
                registry
            )?,

            // Middleware-specific metrics
            middleware_executions: register_int_counter_vec_with_registry!(
                opts!("middleware_executions_total", "Total middleware executions"),
                &[labels::MIDDLEWARE],
                registry
            )?,
            middleware_successes: register_int_counter_vec_with_registry!(
                opts!("middleware_successes_total", "Total middleware successes"),
                &[labels::MIDDLEWARE],
                registry
            )?,
            middleware_failures: register_int_counter_vec_with_registry!(
                opts!("middleware_failures_total", "Total middleware failures"),
                &[labels::MIDDLEWARE],
                registry
            )?,
            middleware_duration: register_histogram_vec_with_registry!(
                format!("{}_middleware_duration_seconds", NAMESPACE),
                "Middleware execution duration",
                &[labels::MIDDLEWARE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Rate limiting metrics
            rate_limit_hits: register_int_counter_vec_with_registry!(
                opts!("rate_limit_hits_total", "Total rate limit hits"),
                &[labels::PATH],
                registry
            )?,
            rate_limit_misses: register_int_counter_vec_with_registry!(
                opts!("rate_limit_misses_total", "Total rate limit misses"),
                &[labels::PATH],
                registry
            )?,
            rate_limit_blocked: register_int_counter_vec_with_registry!(
                opts!("rate_limit_blocked_total", "Total rate limit blocks"),
                &[labels::PATH],
                registry
            )?,

            // Compression metrics
            compression_attempts: register_int_counter_vec_with_registry!(
                opts!("compression_attempts_total", "Total compression attempts"),
                &[labels::COMPRESSION_ALGORITHM],
                registry
            )?,
            compression_successes: register_int_counter_vec_with_registry!(
                opts!("compression_successes_total", "Total compression successes"),
                &[labels::COMPRESSION_ALGORITHM],
                registry
            )?,
            compression_failures: register_int_counter_vec_with_registry!(
                opts!("compression_failures_total", "Total compression failures"),
                &[labels::COMPRESSION_ALGORITHM],
                registry
            )?,
            compression_ratio: register_histogram_vec_with_registry!(
                format!("{}_compression_ratio", NAMESPACE),
                "Compression ratio (compressed/original)",
                &[labels::COMPRESSION_ALGORITHM],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,

            // CORS metrics
            cors_requests: register_int_counter_vec_with_registry!(
                opts!("cors_requests_total", "Total CORS requests"),
                &[labels::ORIGIN, labels::METHOD],
                registry
            )?,
            cors_allowed: register_int_counter_vec_with_registry!(
                opts!("cors_allowed_total", "Total CORS requests allowed"),
                &[labels::ORIGIN, labels::METHOD],
                registry
            )?,
            cors_blocked: register_int_counter_vec_with_registry!(
                opts!("cors_blocked_total", "Total CORS requests blocked"),
                &[labels::ORIGIN, labels::METHOD],
                registry
            )?,

            // Error handling metrics
            error_responses: register_int_counter_vec_with_registry!(
                opts!("error_responses_total", "Total error responses"),
                &[labels::ERROR_TYPE],
                registry
            )?,
            error_types: register_int_counter_vec_with_registry!(
                opts!("error_types_total", "Total error types"),
                &[labels::ERROR_TYPE],
                registry
            )?,
            error_handling_duration: register_histogram_vec_with_registry!(
                format!("{}_error_handling_duration_seconds", NAMESPACE),
                "Error handling duration",
                &[labels::ERROR_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
