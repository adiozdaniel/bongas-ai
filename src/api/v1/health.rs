//! Health check endpoint.

use axum::{routing::get, Json, Router, extract::Extension};
use std::sync::Arc;
use std::time::Instant;

use crate::api::models::HealthResponse;

/// Mount health routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(health_check))
}

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
