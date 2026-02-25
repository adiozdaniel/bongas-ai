//! Health check endpoint.

use axum::{routing::get, Json, Router, extract::Extension};
use std::sync::Arc;
use std::time::Instant;

use crate::api::models::HealthResponse;

/// Mount health routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/live", get(liveness_check))
        .route("/ready", get(readiness_check))
}

/// Liveness probe - determines if the process is alive.
/// Restarts container on failure. Should be very lightweight.
async fn liveness_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "up".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: 0, // Not needed for liveness
    })
}

/// Readiness probe - determines if the app is ready for traffic.
/// Removes from load balancer on failure. Checks dependencies.
async fn readiness_check(
    Extension(engine): Extension<Arc<crate::engine::BongasEngine>>,
    Extension(start_time): Extension<Arc<Instant>>,
) -> Json<HealthResponse> {
    // Check if Postgres is reachable
    let is_db_ready = engine.execution.item_feature_service.pool().check_health().await;
    
    let status = if is_db_ready { "ready" } else { "not_ready" };

    Json(HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    })
}

/// Generic health check for monitoring.
async fn health_check(
    Extension(start_time): Extension<Arc<Instant>>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    })
}
