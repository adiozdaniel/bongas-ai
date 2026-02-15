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
      error_handling::{error_handling_middleware, EnhancedErrorMiddleware, validation_error_middleware},
      metrics::{MetricsCollector, DurationTracker, EndpointMetrics},
      rate_limit::RateLimiter,
  };
  use crate::config::{CompressionConfig, CorsConfig};
  use crate::circuit_breaker::CircuitBreakerRegistry;


pub fn create_router(
    engine: Arc<BongasEngine>,
    redis: Arc<redis::Client>,
    metrics_collector: Arc<MetricsCollector>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
) -> Router {
    // Create endpoint metrics tracker
    let endpoint_metrics = Arc::new(EndpointMetrics::new());
    
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

        // List  scenarios
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

        // ========== FEATURES ENDPOINTS ==========

        // User features
        .route(
            "/api/v1/features/user/:user_id",
            get(v1::features::get_user_features),
        )

        // Item features
        .route(
            "/api/v1/features/item/:item_id",
            get(v1::features::get_item_features),
        )

        // Trending items
        .route(
            "/api/v1/features/trending",
            get(v1::features::get_trending_items),
        )

        // Analytics metrics
        // .route(
        //     "/api/v1/analytics/metrics",
        //     get(v1::analytics::get_analytics_metrics),
        // )
        // .route(
        //     "/api/v1/analytics/metrics/summary",
        //     get(v1::analytics::get_analytics_metrics_summary),
        // )
        // .route(
        //     "/api/v1/analytics/metrics/health",
        //     get(v1::analytics::get_analytics_health),
        // )

        // Health check
        .route("/health", get(|| async { "OK" }))

        // Inject shared state
        .layer(axum::Extension(engine))
        .layer(axum::Extension(redis))
        .layer(axum::Extension(circuit_breaker_registry))
        .layer(axum::Extension(metrics_collector))

        // ========== MIDDLEWARE STACK (applied in reverse order) ==========
        // 1. Validation error handling (outermost)
        .layer(from_fn(validation_error_middleware))
        
        // 2. Enhanced error handling
        .layer(from_fn(EnhancedErrorMiddleware::layer))
        
        // 3. Error handling
        .layer(from_fn(error_handling_middleware))
        
        // 2. Rate limiting
        .layer(from_fn(|req: Request<Body>, next: Next| async move {
            let redis = req.extensions().get::<Arc<redis::Client>>().unwrap();
            let circuit_breaker = req.extensions().get::<Arc<CircuitBreakerRegistry>>().unwrap();
            let rate_limiter = RateLimiter::new(redis.clone(), circuit_breaker.clone(), 60, 100);
            
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
        
        // 4. Response body caching
        
        // 5. Response compression (Enhanced)
        .layer(CompressionConfig::new()
            .min_size(1024)
            .enable_gzip(true)
            .enable_brotli(true)
            .enable_deflate(false)
            .build())
        
        .layer(CorsConfig::dev())

        // 6. Duration tracking middleware
        .layer(from_fn(DurationTracker::layer))
        
        // 7. HTTP metrics middleware (integrates with AnalyticsManager)
        
        // 8. CORS (Enhanced - Development)
        
        // 8. Endpoint metrics tracking
        .layer(from_fn(move |req: Request<Body>, next: Next| {
            endpoint_metrics.clone().layer(req, next)
        }))
        
        // 9. Request tracing
        .layer(TraceLayer::new_for_http())
}