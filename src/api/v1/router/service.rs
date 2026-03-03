//! V1 API Router assembly.
//! This is the single source of truth for all API routes in the system.

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;
use crate::config::AppConfig;
use super::*;

/// Build the complete v1 API router with all domain routes nested under their prefixes.
pub fn routes(config: Arc<AppConfig>) -> Router {
    // ─── STAGE (Public Endpoints) ──────────────────────────────────────────
    
    let public_recommendation_router = Router::new()
        // 1. The Genesis Entry Point: Resolves World & Nav Mesh
        .route("/", get(recommendations::handlers::genesis))
        // 2. The Page Orchestrator: Contextual View Delivery
        .route("/page/{slug}", get(recommendations::handlers::get_page_recommendations))
        // 3. Scenario Detail: Deep Pagination (See All)
        .route("/scenario/{slug}", get(recommendations::handlers::get_scenario_detail))
        // 4. Ingestion: Real-time behavior tracking
        .route("/ingest", post(recommendations::handlers::ingest_activities));

    // ─── BACKSTAGE (Admin Endpoints - System Key Auth) ───────────────────

    let admin_pages_router = Router::new()
        .route("/", post(pages::handlers::save_page_layout))
        .route("/active", get(pages::handlers::list_active_pages))
        .route("/{slug}", get(pages::handlers::get_page_layout).delete(pages::handlers::delete_page_layout));

    let admin_scenarios_router = Router::new()
        .route("/", post(scenarios::handlers::create_scenario).get(scenarios::handlers::list_scenarios))
        .route("/{slug}", get(scenarios::handlers::get_scenario).put(scenarios::handlers::update_scenario).delete(scenarios::handlers::delete_scenario))
        .route("/{slug}/reload", post(scenarios::handlers::reload_scenario))
        .route("/reload-all", post(scenarios::handlers::reload_all_scenarios));

    let admin_features_router = Router::new()
        .route("/user/{user_id}", get(features::handlers::get_user_features))
        .route("/item/{item_id}", get(features::handlers::get_item_features))
        .route("/trending", get(features::handlers::get_trending_items));

    let admin_system_router = Router::new()
        .route("/metrics", get(admin::handlers::get_resilience_metrics))
        .route("/cache/stats", get(admin::handlers::get_cache_stats))
        .route("/ingestion/metrics", get(admin::handlers::get_ingestion_metrics))
        .route("/ingestion/health", get(admin::handlers::get_ingestion_health))
        .route("/models/reload", post(admin::handlers::reload_models))
        .route("/models/stats", get(admin::handlers::get_model_stats))
        .route("/security/status", get(admin::handlers::get_security_status))
        .route("/reload", post(admin::handlers::reload_engine_atomic)) // Global Symphony Refresh
        .route("/suggestions", get(admin::handlers::list_suggestions))
        .route("/suggestions/{id}/approve", post(admin::handlers::approve_suggestion))
        .route("/suggestions/{id}/reject", post(admin::handlers::reject_suggestion))
        .route("/suggestions/{id}/simulate", get(admin::handlers::simulate_suggestion))
        .route("/chatbot/ask", post(admin::handlers::chatbot_ask));
        
    let admin_recommendation_router = Router::new()
        .route("/prewarm", post(recommendations::handlers::prewarm_scenarios));

    let mut admin_router = Router::new()
        .nest("/pages", admin_pages_router)
        .nest("/scenarios", admin_scenarios_router)
        .nest("/features", admin_features_router)
        .nest("/system", admin_system_router)
        .nest("/recommendations", admin_recommendation_router);

    if config.experiments.enabled {
        let experiments_router = Router::new()
            .route("/", get(experiments::handlers::list_experiments));
        admin_router = admin_router.nest("/experiments", experiments_router);
    }

    // ─── HEALTH (System Status) ──────────────────────────────────────────

    let health_router = Router::new()
        .route("/", get(health::handlers::health_check))
        .route("/live", get(health::handlers::liveness_check))
        .route("/ready", get(health::handlers::readiness_check));

    // ─── COMPOSE THE SYMPHONY ────────────────────────────────────────────

    let symphony_router = public_recommendation_router
        .nest("/admin", admin_router);

    Router::new()
        .nest("/recommendation", symphony_router)
        .nest("/health", health_router)
}
