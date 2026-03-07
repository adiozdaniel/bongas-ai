use std::sync::Arc;
use axum::{Router, middleware, http::{Request, HeaderMap}};
use axum::middleware::from_fn;
use axum::extract::Extension;

use crate::engine::coordination::service::BongasEngine;
use crate::resilience::ResilienceMetricsCollector;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::api::ConnectionTracker;

// Middlewares
use crate::middlewares::{
    unified_error_middleware, 
    platform_security_middleware,
    ResilienceMiddleware,
    ResilienceMiddlewareConfig,
    BulkheadMiddleware,
    BulkheadConfig,
    RateLimiter,
    EndpointMetrics,
};

/// Apply all global middleware to the provided router.
pub fn apply_middleware(
    app: Router,
    engine: Arc<BongasEngine>,
    redis: Arc<redis::Client>,
    resilience_metrics: Arc<ResilienceMetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    tracker: Arc<ConnectionTracker>,
) -> Router {
    let engine_clone = engine.clone();
    
    // Initialize Middleware States
    let endpoint_metrics = Arc::new(EndpointMetrics::new());
    
    let resilience_mw = Arc::new(ResilienceMiddleware::with_config(
        circuit_breaker_registry.clone(),
        ResilienceMiddlewareConfig::default(),
    ));

    let bulkhead_mw = Arc::new(BulkheadMiddleware::new(
        BulkheadConfig::default(),
    ));

    let rate_limiter = RateLimiter::new(
        redis,
        circuit_breaker_registry.clone(),
        100, // max requests per window
        60,  // window seconds
        Some(engine.shutdown_tx.subscribe()),
    );

    app
        // 1. Error Handling (Outermost to catch everything)
        .layer(from_fn(unified_error_middleware))
        
        // 2. Metrics & Observability
        .layer(Extension(endpoint_metrics))
        .layer(from_fn(EndpointMetrics::layer))
        
        // 3. Identity (Provide context for downstream)
        .layer(middleware::from_fn(move |req, next| {
            let engine = engine_clone.clone();
            async move {
                crate::api::middleware::identity::service::identity_middleware(req, next, engine).await
            }
        }))

        // 4. Rate Limiting
        .layer(Extension(rate_limiter))
        .layer(from_fn(RateLimiter::layer))

        // 5. Platform Security
        .layer(from_fn(platform_security_middleware))

        // 6. Concurrency & Resilience (Innermost)
        .layer(Extension(bulkhead_mw.clone()))
        .layer(from_fn(BulkheadMiddleware::layer))
        .layer(Extension(resilience_mw.clone()))
        .layer(from_fn(ResilienceMiddleware::layer))
        .layer(middleware::from_fn(crate::api::middleware::adaptive_limiter::service::adaptive_limiter_middleware))
        .layer(Extension(tracker))
        .layer(Extension(circuit_breaker_registry))
        .layer(Extension(resilience_metrics))
}

pub fn extract_request_id<B>(req: &Request<B>) -> String {
    extract_request_id_from_headers(req.headers())
}

pub fn extract_request_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}
