//! Recommendation endpoints and handlers.

pub mod handlers;
pub mod service;

use axum::{
    routing::get,
    Router,
};
use self::handlers::*;

/// Mount all recommendation routes.
pub fn routes() -> Router {
    Router::new()
        .route("/home/{user_id}", get(get_home_recommendations))
        .route("/continue-watching/{user_id}", get(get_continue_watching))
        .route("/trending", get(get_trending))
        .route("/because-you-watched/{user_id}/{item_id}", get(get_because_you_watched))
        .route("/genre/{genre}/{user_id}", get(get_genre_recommendations))
        .route("/new-releases/{user_id}", get(get_new_releases))
        .route("/live-tv/{user_id}", get(get_live_tv))
        .route("/page/{page_slug}/{user_id}", get(get_page_recommendations))
        .route("/{scenario_slug}/{user_id}", get(get_recommendations))
}
