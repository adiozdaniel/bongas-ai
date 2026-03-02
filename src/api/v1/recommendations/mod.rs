//! Recommendation endpoints and handlers.

pub mod handlers;
pub mod service;

use axum::{
    routing::get,
    Router,
};
use self::handlers::*;

/// Mount all recommendation routes.
/// The Symphony Mesh: Everything is a Page.
pub fn routes() -> Router {
    Router::new()
        // The Master Orchestrator: Unified Page Delivery
        .route("/page/{page_slug}/{user_id}", get(get_page_recommendations))
}
