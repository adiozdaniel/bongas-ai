//! API router construction logic.

use axum::Router;
use std::sync::Arc;
use std::time::Instant;

use crate::engine::BongasEngine;
use crate::config::AppConfig;
use crate::middlewares::metrics::MetricsCollector;
use crate::middlewares::rate_limit::RateLimiter;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::api::v1;
use crate::api::middleware;
use crate::api::middleware::adaptive_limiter::ConnectionTracker;

use axum::routing::get;

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

    // Adaptive Rate Limiter: Max 3 concurrent SSE connections per visitor (Shield)
    let connection_tracker = Arc::new(ConnectionTracker::new(3));

    let routes = Router::new()
        .route("/metrics", get(v1::admin::get_resilience_metrics))
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
        connection_tracker,
        start_time
    )
}
