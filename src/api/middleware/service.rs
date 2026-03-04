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
    response::{Response, IntoResponse},
    http::{StatusCode, HeaderMap},
    Json,
};
use std::sync::Arc;
use tower_http::{
    trace::TraceLayer,
    request_id::{SetRequestIdLayer, MakeRequestUuid, RequestId},
};
use tracing::{error, info_span};
use uuid::Uuid;

use crate::api::models::RecommendationItem;
use crate::middlewares::{
    unified_error::unified_error_middleware,
    metrics::{DurationTracker, EndpointMetrics},
    rate_limit::{RateLimiter, RateLimitResult},
    resilience::ResilienceMiddleware,
    bulkhead::BulkheadMiddleware,
};
use crate::config::{CompressionConfig, CorsConfig};
use crate::circuit_breaker::CircuitBreakerRegistry;

use crate::middlewares::metrics::MetricsCollector;
use crate::engine::coordination::service::BongasEngine;
use crate::config::AppConfig;
use std::time::Instant;

use crate::api::middleware::identity::identity_middleware;
use crate::api::middleware::identity::IdentityContext;
use crate::api::middleware::adaptive_limiter::{adaptive_limiter_middleware, ConnectionTracker};

/// Apply the full middleware stack to a router.
///
/// Middleware is applied in reverse order (bottom to top):
/// 11. Set Request ID (Outermost wrapper)
/// 10. Adaptive Rate Limiting (SSE protection per visitor)
/// 9. Identity & Visitor Persistence (Zero-Touch)
/// 8. Request Tracing (Creates Span with Request ID & Identity)
/// 7. Unified Error Handling (Catches everything below, uses Request ID)
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
    connection_tracker: Arc<ConnectionTracker>,
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

        // 6. Unified Error Handling (Catches errors from all inner layers)
        .layer(from_fn(unified_error_middleware))

        // 7. Request Tracing (Correlated via Request ID & Identity)
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request: &Request<Body>| {
                let request_id = request.extensions()
                    .get::<RequestId>()
                    .map(|id| id.header_value().to_str().unwrap_or("unknown"))
                    .unwrap_or("unknown");
                
                let visitor_id = request.extensions()
                    .get::<IdentityContext>()
                    .map(|id| id.visitor_id.as_str())
                    .unwrap_or("unknown");
                
                info_span!(
                    "http_request",
                    request_id = %request_id,
                    visitor_id = %visitor_id,
                    method = %request.method(),
                    uri = %request.uri(),
                )
            })
        )

        // 8. Identity & Visitor Persistence (Zero-Touch)
        .layer(from_fn(identity_middleware))

        // 9. Adaptive Rate Limiting
        .layer(from_fn(adaptive_limiter_middleware))

        // 10. Extension Injection (Available to all of the above)
        .layer(axum::Extension(engine))
        .layer(axum::Extension(config))
        .layer(axum::Extension(redis))
        .layer(axum::Extension(rate_limiter))
        .layer(axum::Extension(circuit_breaker_registry))
        .layer(axum::Extension(metrics_collector))
        .layer(axum::Extension(connection_tracker))
        .layer(axum::Extension(start_time))

        // 11. Set Request ID (Outermost - runs FIRST)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

/// Helper to extract RequestId from request extensions
pub fn extract_request_id(req: &Request<Body>) -> String {
    req.extensions()
        .get::<RequestId>()
        .map(|id| id.header_value().to_str().unwrap_or("unknown").to_string())
        .or_else(|| {
            extract_request_id_from_headers(req.headers()).into()
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Helper to extract request id from HeaderMap
pub fn extract_request_id_from_headers(headers: &HeaderMap) -> String {
    headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Rate-limit middleware extracted as a named function for readability.
async fn rate_limit_layer(req: Request<Body>, next: Next) -> Response {
    let request_id = extract_request_id(&req);
    let rate_limiter = match req.extensions().get::<Arc<RateLimiter>>() {
        Some(rl) => rl.clone(),
        None => {
            error!(request_id = %request_id, "RateLimiter extension missing");
            let response = crate::api::models::StandardResponse::<()>::error(
                "Internal configuration error",
                "INTERNAL_ERROR",
                "Internal",
                false,
            ).with_request_id(request_id);
            
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ip = req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    match rate_limiter.check(&ip).await {
        RateLimitResult::Allowed => next.run(req).await,
        RateLimitResult::ShadowBan => {
            let response = crate::api::models::StandardResponse::success(Vec::<RecommendationItem>::new())
                .with_request_id(request_id);
            (StatusCode::OK, Json(response)).into_response()
        }
        RateLimitResult::RateLimited(status) => {
            let response = crate::api::models::StandardResponse::<()>::error(
                "Too many requests",
                "RATE_LIMIT_EXCEEDED",
                "Overload",
                true,
            )
            .with_request_id(request_id)
            .with_retry_after(status.reset_in_seconds * 1000);

            (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("X-RateLimit-Limit", status.limit.to_string()),
                    ("X-RateLimit-Remaining", "0".to_string()),
                    ("X-RateLimit-Reset", status.window_seconds.to_string()),
                    ("Retry-After", status.reset_in_seconds.to_string()),
                ],
                Json(response),
            ).into_response()
        }
    }
}
