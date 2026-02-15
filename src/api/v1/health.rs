//! Health check endpoint.

use axum::{routing::get, Json, Router};

use crate::api::models::HealthResponse;

/// Mount health routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(health_check))
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: 0, // TODO: Implement actual uptime tracking
    })
}
