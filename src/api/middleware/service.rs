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
    _engine: Arc<BongasEngine>,
    config: Arc<AppConfig>,
    _redis: Arc<redis::Client>,
    _resilience_metrics: Arc<ResilienceMetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    tracker: Arc<ConnectionTracker>,
    _start_time: Arc<Instant>,
) -> Router {
    app.layer(middleware::from_fn(move |req: Request<axum::body::Body>, next: Next| {
        let _breakers = circuit_breaker_registry.clone();
        let _tracker = tracker.clone();
        let _env = config.server.environment.clone();
        
        async move {
            // Identity & Rate Limiting Middleware
            let response = next.run(req).await;
            
            // Record generic metrics
            // metrics.record_api_call(&env);
            
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
