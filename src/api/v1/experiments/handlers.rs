//! Experimentation endpoints (A/B Testing & Bandits)

use axum::{routing::get, Router, Json};
use crate::api::models::StandardResponse;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_experiments))
}

async fn list_experiments() -> Json<StandardResponse<serde_json::Value>> {
    Json(StandardResponse::success(serde_json::json!({
        "status": "active",
        "message": "Experimentation system is active."
    })))
}
