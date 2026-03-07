//! Logic for creating and configuring the main application router.

use std::sync::Arc;
use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::AppConfig;
use crate::engine::coordination::service::BongasEngine;
use crate::resilience::ResilienceMetricsCollector;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::api::v1;
use crate::api::ConnectionTracker;

/// Create the main application router with all routes and middleware.
pub fn create_router(
    engine: Arc<BongasEngine>,
    config: Arc<AppConfig>,
    redis: Arc<redis::Client>,
    resilience_metrics: Arc<ResilienceMetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    tracker: Arc<ConnectionTracker>,
) -> Router {
    // 1. Build V1 Routes
    let v1_routes = v1::router::service::routes(engine.clone(), config.clone());

    // 2. Build Global Router
    let app = Router::new()
        .nest("/api/v1", v1_routes)
        .route("/health/live", get(v1::pulse::health::service::liveness_check))
        .route("/health/ready", get(v1::pulse::health::service::readiness_check))
        .layer(TraceLayer::new_for_http());

    // 3. Apply Global Middleware
    crate::api::middleware::service::apply_middleware(
        app,
        engine,
        redis,
        resilience_metrics,
        circuit_breaker_registry,
        tracker,
    )
}
