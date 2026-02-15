//! API layer — thin composition of versioned routes, middleware, and shared state.

pub mod v1;
pub mod models;
pub mod middleware;

use axum::Router;
use std::sync::Arc;

use crate::engine::BongasEngine;
use crate::middlewares::metrics::MetricsCollector;
use crate::circuit_breaker::CircuitBreakerRegistry;

/// Build the complete API router with routes, shared state, and middleware.
pub fn create_router(
    engine: Arc<BongasEngine>,
    redis: Arc<redis::Client>,
    metrics_collector: Arc<MetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
) -> Router {
    let routes = Router::new()
        .nest("/api/v1", v1::routes())
        .nest("/health", v1::health::routes())
        // Inject shared state
        .layer(axum::Extension(engine))
        .layer(axum::Extension(redis))
        .layer(axum::Extension(circuit_breaker_registry.clone()))
        .layer(axum::Extension(metrics_collector));

    middleware::apply_middleware(routes, circuit_breaker_registry)
}
