//! V1 API — composes domain sub-routers into a single versioned router.

pub mod recommendations;
pub mod scenarios;
pub mod features;
pub mod health;
pub mod admin;

use axum::Router;

/// Build the complete v1 API router with all domain routes nested under their prefixes.
pub fn routes() -> Router {
    Router::new()
        .nest("/recommendations", recommendations::routes())
        .nest("/scenarios", scenarios::routes())
        .nest("/features", features::routes())
        .nest("/admin", admin::routes())
}
