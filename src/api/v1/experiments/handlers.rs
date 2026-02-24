//! Experimentation endpoints (A/B Testing & Bandits)

use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_experiments))
}

async fn list_experiments() -> &'static str {
    "Experimentation system is active."
}
