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
use tower_http::trace::TraceLayer;
use serde_json::json;

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

/// Apply the full middleware stack to a router.
///
/// Middleware is applied in reverse order (outermost layer listed first):
/// 1. Unified error handling
/// 2. Resilience (circuit breaker)
/// 3. Bulkhead (concurrency limiting)
/// 4. Rate limiting
/// 5. Compression
/// 6. CORS
/// 7. Duration tracking
/// 8. Endpoint metrics
/// 9. Request tracing
pub fn apply_middleware(
    router: Router,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
) -> Router {
    let endpoint_metrics = Arc::new(EndpointMetrics::new());
    let resilience_middleware = Arc::new(ResilienceMiddleware::new(circuit_breaker_registry));
    let bulkhead_middleware = Arc::new(BulkheadMiddleware::with_defaults());

    router
        // 1. Unified error handling (outermost — catches all errors)
        .layer(from_fn(unified_error_middleware))

        // 1.1 Platform Security (Checks X-Platform headers)
        .layer(from_fn(platform_security_middleware))

        // 2. Resilience middleware (circuit breaker)
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            let mw = Arc::clone(&resilience_middleware);
            async move {
                let state = axum::extract::State(mw);
                ResilienceMiddleware::layer(state, req, next).await
            }
        }))

        // 3. Bulkhead middleware (concurrency limiting)
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            let mw = Arc::clone(&bulkhead_middleware);
            async move {
                let state = axum::extract::State(mw);
                BulkheadMiddleware::layer(state, req, next).await
            }
        }))

        // 4. Rate limiting
        .layer(from_fn(rate_limit_layer))

        // 5. Response compression
        .layer(CompressionConfig::new()
            .min_size(1024)
            .enable_gzip(true)
            .enable_brotli(true)
            .enable_deflate(false)
            .build())

        // 6. CORS
        .layer(CorsConfig::dev())

        // 7. Duration tracking
        .layer(from_fn(DurationTracker::layer))

        // 8. Endpoint metrics
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            endpoint_metrics.clone().layer(req, next)
        }))

        // 9. Request tracing
        .layer(TraceLayer::new_for_http())
}

/// Rate-limit middleware extracted as a named function for readability.
async fn rate_limit_layer(req: Request<Body>, next: Next) -> Response {
    let rate_limiter = req.extensions().get::<Arc<RateLimiter>>().expect("RateLimiter extension missing").clone();

    // Use peer addr as primary IP source to prevent spoofing via X-Forwarded-For
    // In production, this should only trust X-Forwarded-For if it comes from a known proxy CIDR.
    let ip = req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("X-Forwarded-For")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".to_string());

    match rate_limiter.check(&ip).await {
        RateLimitResult::Allowed => next.run(req).await,
        RateLimitResult::ShadowBan => {
            // Shadow Ban: Return 200 OK with a generic/empty feed to mislead bots
            // Removed X-Bongas-Status header as it defeats the purpose of shadow banning
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "success": true,
                    "data": [],
                    "message": "Recommendations refreshed",
                    "metadata": {
                        "count": 0,
                        "source": "cache",
                        "request_id": uuid::Uuid::new_v4().to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
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
                    "error": "Rate limit exceeded",
                    "message": format!(
                        "Too many requests. Limit: {} requests per {} seconds",
                        status.limit, status.window_seconds
                    ),
                    "limit": status.limit,
                    "remaining": 0,
                    "reset_time": status.reset_in_seconds,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }).to_string()))
                .unwrap()
        }
    }
}
