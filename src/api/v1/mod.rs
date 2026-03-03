//! V1 API — composes domain sub-routers into a single versioned router.
//! The Bongas-AI Symphony: Strict separation between Stage (Public) and Backstage (Admin).

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
    // 1. BACKSTAGE: Administrative routes (System Key Auth)
    let mut admin_router = Router::new()
        .nest("/pages", pages::routes())
        .nest("/scenarios", scenarios::routes())
        .nest("/features", features::routes())
        .nest("/system", admin::routes()); // renamed from metrics to system

    if config.experiments.enabled {
        admin_router = admin_router.nest("/experiments", experiments::routes());
    }

    // 2. THE STAGE: Public content delivery and feedback
    let public_router = recommendations::routes();

    // 3. COMPOSE: Combine into unified recommendation prefix
    let symphony_router = public_router.nest("/admin", admin_router);

    Router::new()
        .nest("/recommendation", symphony_router)
        .nest("/health", health::routes())
}
