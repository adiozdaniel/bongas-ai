//! API layer — thin composition of versioned routes, middleware, and shared state.

pub mod v1;
pub mod models;
pub mod middleware;

use axum::Router;
use std::sync::Arc;
use std::time::Instant;

use crate::engine::BongasEngine;
use crate::config::AppConfig;
use crate::middlewares::metrics::MetricsCollector;
use crate::middlewares::rate_limit::RateLimiter;
use crate::circuit_breaker::CircuitBreakerRegistry;

/// Build the complete API router with routes, shared state, and middleware.
pub fn create_router(
    engine: Arc<BongasEngine>,
    config: Arc<AppConfig>,
    redis: Arc<redis::Client>,
    metrics_collector: Arc<MetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    start_time: Arc<Instant>,
) -> Router {
    let rate_limiter = RateLimiter::new(
        redis.clone(),
        circuit_breaker_registry.clone(),
        100, // max requests per minute (L2 threshold)
        60,  // window seconds
        Some(engine.shutdown_tx.subscribe()),
    );

    let routes = Router::new()
        .nest("/api/v1", v1::routes(config.clone()))
        .nest("/health", v1::health::routes());

    middleware::apply_middleware(
        routes, 
        circuit_breaker_registry,
        engine,
        config,
        redis,
        rate_limiter,
        metrics_collector,
        start_time
    )
}
