//! Recommendation endpoints and handlers.

pub mod handlers;
pub mod service;

use axum::{
    routing::{get, post},
    Router,
};
use self::handlers::*;

/// Mount all recommendation routes.
/// The Symphony Mesh: Everything is a Page.
pub fn routes() -> Router {
    Router::new()
        // The Master Orchestrator: Unified Page Delivery
        .route("/page/{page_slug}/{user_id}", get(get_page_recommendations))
        // Predictive Warming: Triggered by scroll depth (The Shield)
        .route("/prewarm", post(prewarm_scenarios))
}
