use std::sync::Arc;
use tokio::time::Instant;
use axum::{Router, middleware, http::{Request, HeaderMap}};
use axum::middleware::Next;

use crate::AppConfig;
use crate::engine::coordination::service::BongasEngine;
use crate::resilience::ResilienceMetricsCollector;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::api::ConnectionTracker;

/// Apply all global middleware to the provided router.
pub fn apply_middleware(
    app: Router,
    engine: Arc<BongasEngine>,
    config: Arc<AppConfig>,
    _redis: Arc<redis::Client>,
    _resilience_metrics: Arc<ResilienceMetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    tracker: Arc<ConnectionTracker>,
    _start_time: Arc<Instant>,
) -> Router {
    let engine_clone = engine.clone();
    let tracker_clone = tracker.clone();
    let circuit_breaker_registry_clone = circuit_breaker_registry.clone();
    let environment = config.server.environment.clone();
    
    app
        .layer(middleware::from_fn(move |req, next| {
            let engine = engine_clone.clone();
            async move {
                crate::api::middleware::identity::service::identity_middleware(req, next, engine).await
            }
        }))
        .layer(middleware::from_fn(crate::api::middleware::adaptive_limiter::service::adaptive_limiter_middleware))
        .layer(middleware::from_fn(move |req: Request<axum::body::Body>, next: Next| {
            let _breakers = circuit_breaker_registry_clone.clone();
            let _tracker = tracker_clone.clone();
            let _env = environment.clone();
            
            async move {
                let response = next.run(req).await;
                response
            }
        }))
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
