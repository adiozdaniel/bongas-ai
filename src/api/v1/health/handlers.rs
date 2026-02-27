//! Health check endpoint.

use axum::{routing::get, Json, Router, extract::Extension};
use std::sync::Arc;
use std::time::Instant;

use crate::api::models::{HealthResponse, StandardResponse};

/// Mount health routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/live", get(liveness_check))
        .route("/ready", get(readiness_check))
}

use tower_http::request_id::RequestId;

/// Liveness probe - determines if the process is alive.
/// Restarts container on failure. Should be very lightweight.
async fn liveness_check(
    Extension(start_time): Extension<Arc<Instant>>,
    Extension(request_id): Extension<RequestId>,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let data = HealthResponse {
        status: "up".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}

/// Readiness probe - determines if the app is ready for traffic.
/// Removes from load balancer on failure. Checks dependencies.
async fn readiness_check(
    Extension(engine): Extension<Arc<crate::engine::BongasEngine>>,
    Extension(start_time): Extension<Arc<Instant>>,
    Extension(request_id): Extension<RequestId>,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    // Check if Postgres is reachable
    let is_db_ready = engine.execution.item_feature_service.pool().check_health().await;
    
    let status = if is_db_ready { "ready" } else { "not_ready" };

    let data = HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}

/// Generic health check for monitoring.
async fn health_check(
    Extension(start_time): Extension<Arc<Instant>>,
    Extension(request_id): Extension<RequestId>,
) -> Json<StandardResponse<HealthResponse>> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let data = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: start_time.elapsed().as_secs(),
    };
    Json(StandardResponse::success(data).with_request_id(request_id))
}
