//! Middleware stack assembly for the API router.
//!
//! Extracts all middleware wiring out of the router builder so route
//! definitions stay clean and middleware ordering is managed in one place.

use axum::{
    Router,
    middleware::from_fn,
    extract::Request,
    body::Body,
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use std::sync::Arc;
use tower_http::{
    trace::TraceLayer,
    request_id::{SetRequestIdLayer, MakeRequestUuid, RequestId},
};
use serde_json::json;
use tracing::{error, info_span};
use uuid::Uuid;

use crate::middlewares::{
    unified_error::unified_error_middleware,
    metrics::{DurationTracker, EndpointMetrics},
    rate_limit::{RateLimiter, RateLimitResult},
    resilience::ResilienceMiddleware,
    bulkhead::BulkheadMiddleware,
    platform_security::platform_security_middleware,
};
use crate::config::{CompressionConfig, CorsConfig};
use crate::circuit_breaker::CircuitBreakerRegistry;

use crate::middlewares::metrics::MetricsCollector;
use crate::engine::BongasEngine;
use crate::config::AppConfig;
use std::time::Instant;

/// Apply the full middleware stack to a router.
///
/// Middleware is applied in reverse order (bottom to top):
/// 10. Set Request ID (Outermost wrapper)
/// 9. Unified Error Handling (Catches everything below, uses Request ID)
/// 8. Platform Security
/// 7. Request Tracing (Creates Span with Request ID)
/// 6. Resilience (Circuit Breakers)
/// 5. Bulkhead
/// 4. Rate Limiting
/// 3. Compression / CORS
/// 2. Metrics / Duration
/// 1. Extension injection (Innermost - available to all above)
#[allow(clippy::too_many_arguments)]
pub fn apply_middleware(
    router: Router,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    engine: Arc<BongasEngine>,
    config: Arc<AppConfig>,
    redis: Arc<redis::Client>,
    rate_limiter: Arc<RateLimiter>,
    metrics_collector: Arc<MetricsCollector>,
    start_time: Arc<Instant>,
) -> Router {
    let endpoint_metrics = Arc::new(EndpointMetrics::new());
    let resilience_middleware = Arc::new(ResilienceMiddleware::new(circuit_breaker_registry.clone()));
    let bulkhead_middleware = Arc::new(BulkheadMiddleware::with_defaults());

    router
        // 1. Metrics & Duration Tracking (Innermost - closest to handler)
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            endpoint_metrics.clone().layer(req, next)
        }))
        .layer(from_fn(DurationTracker::layer))

        // 2. Infra (Compression, CORS)
        .layer(CorsConfig::dev())
        .layer(CompressionConfig::new()
            .min_size(1024)
            .enable_gzip(true)
            .build())

        // 3. Rate Limiting (Needs RateLimiter extension)
        .layer(from_fn(rate_limit_layer))

        // 4. Bulkhead (Needs BulkheadMiddleware extension)
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            let mw = Arc::clone(&bulkhead_middleware);
            async move {
                let state = axum::extract::Extension(mw);
                BulkheadMiddleware::layer(state, req, next).await
            }
        }))

        // 5. Resilience (Needs ResilienceMiddleware extension)
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            let mw = Arc::clone(&resilience_middleware);
            async move {
                let state = axum::extract::Extension(mw);
                ResilienceMiddleware::layer(state, req, next).await
            }
        }))

        // 6. Platform Security (Needs AppConfig extension)
        .layer(from_fn(platform_security_middleware))

        // 7. Unified Error Handling (Catches errors from all inner layers)
        .layer(from_fn(unified_error_middleware))

        // 8. Request Tracing (Correlated via Request ID)
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request: &Request<Body>| {
                let request_id = request.extensions()
                    .get::<RequestId>()
                    .map(|id| id.header_value().to_str().unwrap_or("unknown"))
                    .unwrap_or("unknown");
                
                info_span!(
                    "http_request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri(),
                )
            })
        )

        // 9. Extension Injection (Available to all of the above)
        .layer(axum::Extension(engine))
        .layer(axum::Extension(config))
        .layer(axum::Extension(redis))
        .layer(axum::Extension(rate_limiter))
        .layer(axum::Extension(circuit_breaker_registry))
        .layer(axum::Extension(metrics_collector))
        .layer(axum::Extension(start_time))

        // 10. Set Request ID (Outermost - runs FIRST)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

/// Helper to extract RequestId from request extensions
pub fn extract_request_id(req: &Request<Body>) -> String {
    req.extensions()
        .get::<RequestId>()
        .map(|id| id.header_value().to_str().unwrap_or("unknown").to_string())
        .or_else(|| {
            req.headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Rate-limit middleware extracted as a named function for readability.
async fn rate_limit_layer(req: Request<Body>, next: Next) -> Response {
    let request_id = extract_request_id(&req);
    let rate_limiter = match req.extensions().get::<Arc<RateLimiter>>() {
        Some(rl) => rl.clone(),
        None => {
            error!(request_id = %request_id, "RateLimiter extension missing");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(json!({
                    "success": false,
                    "error": {
                        "message": "Internal configuration error",
                        "code": "INTERNAL_ERROR",
                        "classification": "Internal",
                        "retriable": false,
                    },
                    "meta": {
                        "request_id": request_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }).to_string()))
                .unwrap();
        }
    };

    let ip = req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    match rate_limiter.check(&ip).await {
        RateLimitResult::Allowed => next.run(req).await,
        RateLimitResult::ShadowBan => {
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "success": true,
                    "data": [],
                    "meta": {
                        "request_id": request_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }).to_string()))
                .unwrap()
        }
        RateLimitResult::RateLimited(status) => {
            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("X-RateLimit-Limit", status.limit.to_string())
                .header("X-RateLimit-Remaining", "0")
                .header("X-RateLimit-Reset", status.window_seconds.to_string())
                .header("Retry-After", status.reset_in_seconds.to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "success": false,
                    "error": {
                        "message": "Too many requests",
                        "code": "RATE_LIMIT_EXCEEDED",
                        "classification": "Overload",
                        "retriable": true,
                    },
                    "meta": {
                        "request_id": request_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }).to_string()))
                .unwrap()
        }
    }
}
