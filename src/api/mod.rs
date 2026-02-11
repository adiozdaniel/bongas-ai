pub mod v1;
pub mod error;
pub mod models;

use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware::from_fn,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer, compression::CompressionLayer};
use tower_http::compression::CompressionLevel;
use crate::engine::BongasEngine;
use crate::middlewares::{
    logging::logging_middleware,
    error_handling::error_handling_middleware,
    metrics::MetricsCollector,
};

pub fn create_router(
    engine: Arc<BongasEngine>,
    redis: Arc<redis::Client>,
    metrics_collector: Arc<MetricsCollector>,
) -> Router {
    Router::new()
        // ========== EXISTING V1 ENDPOINTS (PRESERVED) ==========

        // Home recommendations
        .route(
            "/api/v1/recommendations/home/:user_id",
            get(v1::handlers::recommendations::get_home_recommendations),
        )

        // Continue watching
        .route(
            "/api/v1/recommendations/continue-watching/:user_id",
            get(v1::handlers::recommendations::get_continue_watching),
        )

        // Trending
        .route(
            "/api/v1/recommendations/trending",
            get(v1::handlers::recommendations::get_trending),
        )

        // Because you watched
        .route(
            "/api/v1/recommendations/because-you-watched/:user_id/:item_id",
            get(v1::handlers::recommendations::get_because_you_watched),
        )

        // Genre recommendations
        .route(
            "/api/v1/recommendations/genre/:genre/:user_id",
            get(v1::handlers::recommendations::get_genre_recommendations),
        )

        // New releases
        .route(
            "/api/v1/recommendations/new-releases/:user_id",
            get(v1::handlers::recommendations::get_new_releases),
        )

        // Live TV
        .route(
            "/api/v1/recommendations/live-tv/:user_id",
            get(v1::handlers::recommendations::get_live_tv),
        )

        // ========== NEW: DYNAMIC SCENARIO MANAGEMENT ==========

        // Create scenario
        .route(
            "/api/v1/scenarios",
            post(v1::handlers::scenarios::create_scenario),
        )

        // List all scenarios
        .route(
            "/api/v1/scenarios",
            get(v1::handlers::scenarios::list_scenarios),
        )

        // Get scenario details
        .route(
            "/api/v1/scenarios/:slug",
            get(v1::handlers::scenarios::get_scenario),
        )

        // Update scenario
        .route(
            "/api/v1/scenarios/:slug",
            put(v1::handlers::scenarios::update_scenario),
        )

        // Delete scenario
        .route(
            "/api/v1/scenarios/:slug",
            delete(v1::handlers::scenarios::delete_scenario),
        )

        // Hot-reload single scenario
        .route(
            "/api/v1/scenarios/:slug/reload",
            post(v1::handlers::scenarios::reload_scenario),
        )

        // Hot-reload all scenarios
        .route(
            "/api/v1/scenarios/reload-all",
            post(v1::handlers::scenarios::reload_all_scenarios),
        )

        // ========== ADMIN ENDPOINTS ==========

        // Cache stats
        .route(
            "/api/v1/admin/cache-stats",
            get(v1::handlers::admin::get_cache_stats),
        )

        // Invalidate cache
        .route(
            "/api/v1/admin/cache/invalidate",
            post(v1::handlers::admin::invalidate_cache),
        )

        // Kafka metrics
        .route(
            "/api/v1/admin/kafka/metrics",
            get(v1::handlers::admin::get_kafka_metrics),
        )

        // Kafka health
        .route(
            "/api/v1/admin/kafka/health",
            get(v1::handlers::admin::get_kafka_health),
        )

        // Health check
        .route("/health", get(v1::handlers::admin::health_check))

        // Inject shared state
        .layer(axum::Extension(engine))
        .layer(axum::Extension(redis))
        .layer(axum::Extension(metrics_collector))

        // ========== MIDDLEWARE STACK (applied in reverse order) ==========
        // 1. Error handling (outermost)
        .layer(from_fn(error_handling_middleware))
        
        // 2. Request logging with correlation IDs
        .layer(from_fn(logging_middleware))
        
        // 3. Response compression (Gzip)
        .layer(CompressionLayer::new().gzip(true).quality(CompressionLevel::Fastest))
        
        // 4. CORS (already configured)
        .layer(CorsLayer::permissive())
        
        // 5. Request tracing
        .layer(TraceLayer::new_for_http())
}
