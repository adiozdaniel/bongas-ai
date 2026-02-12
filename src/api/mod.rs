pub mod v1;
pub mod error;
pub mod models;

use axum::{
    routing::{get, post, put, delete},
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
use crate::engine::BongasEngine;
use crate::middlewares::{
    logging::logging_middleware,
    error_handling::error_handling_middleware,
    metrics::MetricsCollector,
    cors::create_dev_cors_layer,
    compression::CompressionConfig,
    rate_limit::RateLimiter,
    response_cache::ResponseCacheMiddleware,
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

        // ONNX model hot-reload
        .route(
            "/api/v1/admin/models/reload",
            post(v1::handlers::admin::reload_models),
        )

        // ONNX model stats
        .route(
            "/api/v1/admin/models/stats",
            get(v1::handlers::admin::get_model_stats),
        )

        // Security status
        .route(
            "/api/v1/admin/security/status",
            get(v1::handlers::admin::get_security_status),
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
        
        // 2. Rate limiting
        .layer(from_fn(|req: Request<Body>, next: Next| async move {
            let redis = req.extensions().get::<Arc<redis::Client>>().unwrap();
            let rate_limiter = RateLimiter::new(redis.clone(), 100, 60);
            
            // Extract IP from request
            let ip = req.headers()
                .get("X-Forwarded-For")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("127.0.0.1");
            
            // Check rate limit status
            match rate_limiter.get_status(ip).await {
                Ok(status) if status.is_limited => {
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
                            "message": format!("Too many requests. Limit: {} requests per {} seconds", status.limit, status.window_seconds),
                            "limit": status.limit,
                            "remaining": 0,
                            "reset_time": status.reset_in_seconds,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }).to_string()))
                        .unwrap()
                }
                Ok(_) => next.run(req).await,
                Err(_) => next.run(req).await,
            }
        }))
        
        // 3. Request logging with correlation IDs
        .layer(from_fn(logging_middleware))
        
        // 4. Response body caching
        .layer(from_fn(|req: Request<Body>, next: Next| async move {
            let redis = req.extensions().get::<Arc<redis::Client>>().cloned();
            match redis {
                Some(redis) => {
                    let cache = ResponseCacheMiddleware::new(redis, 300); // 5 minutes TTL
                    cache.layer(req, next).await.unwrap_or_else(|e| {
                        Response::builder()
                            .status(e)
                            .body(Body::empty())
                            .unwrap()
                    })
                }
                None => next.run(req).await
            }
        }))
        
        // 5. Response compression (Enhanced)
        .layer(CompressionConfig::new()
            .min_size(1024)
            .enable_gzip(true)
            .enable_brotli(true)
            .enable_deflate(false)
            .build())
        
        // 6. CORS (Enhanced - Development)
        .layer(create_dev_cors_layer())
        
        // 7. Request tracing
        .layer(TraceLayer::new_for_http())
}
