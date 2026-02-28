//! V1 API — composes domain sub-routers into a single versioned router.

pub mod recommendations;
pub mod scenarios;
pub mod pages;
pub mod features;
pub mod health;
pub mod admin;
pub mod experiments;

use axum::Router;
use std::sync::Arc;
use crate::config::AppConfig;

/// Build the complete v1 API router with all domain routes nested under their prefixes.
pub fn routes(config: Arc<AppConfig>) -> Router {
    let mut router = Router::new()
        .nest("/recommendations", recommendations::routes())
        .nest("/scenarios", scenarios::routes())
        .nest("/pages", pages::routes())
        .nest("/features", features::routes())
        .nest("/admin", admin::routes());

    if config.experiments.enabled {
        router = router.nest("/experiments", experiments::routes());
    }

    router
}
