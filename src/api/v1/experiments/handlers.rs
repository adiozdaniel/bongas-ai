//! Experimentation endpoints (A/B Testing & Bandits)

use axum::{routing::get, Router, Json, extract::Extension};
use crate::api::models::StandardResponse;
use tower_http::request_id::RequestId;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_experiments))
}

async fn list_experiments(Extension(request_id): Extension<RequestId>) -> Json<StandardResponse<serde_json::Value>> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    Json(StandardResponse::success(serde_json::json!({
        "status": "active",
        "message": "Experimentation system is active."
    })).with_request_id(request_id))
}
