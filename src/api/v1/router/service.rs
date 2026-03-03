//! V1 API Router assembly.
//! The Grand Composer: Context-Aware Routing for the Bongas-AI Symphony.

use axum::{
    routing::{get, post},
    Router,
    middleware::from_fn,
};
use std::sync::Arc;
use crate::config::{AppConfig, CompressionConfig};
use crate::engine::BongasEngine;
use super::*;

/// Build the complete v1 API router with context-aware infrastructure logic.
pub fn routes(config: Arc<AppConfig>, engine: Arc<BongasEngine>) -> Router {
    
    // ─── THE PULSE (Operations & Observability) ──────────────────────────
    // Infrastructure routes: Low overhead, no compression needed.
    let pulse_router = Router::new()
        .route("/health/live", get(pulse::health::liveness_check))
        .route("/health/ready", get(pulse::health::readiness_check))
        .route("/system/metrics", get(pulse::metrics::get_resilience_metrics))
        .route("/system/cache/stats", get(pulse::metrics::get_cache_stats))
        .route("/system/ingestion/health", get(pulse::metrics::get_ingestion_health));

    // ─── THE STAGE (Public Discovery) ────────────────────────────────────
    // Dynamic content delivery.
    
    // 1. Streaming Layer (No Compression to avoid buffering delay)
    let stage_streaming = Router::new()
        .route("/", get(stage::discovery::genesis))
        .route("/page/{slug}", get(stage::discovery::get_page_recommendations));

    // 2. Data Layer (JSON-heavy, Compression enabled)
    let stage_data = Router::new()
        .route("/scenario/{slug}", get(stage::discovery::get_scenario_detail))
        .route("/ingest", post(stage::ingestion::ingest_activities))
        .layer(CompressionConfig::new().build());

    let stage_router = stage_streaming.merge(stage_data);

    // ─── THE BACKSTAGE (Admin Control) ──────────────────────────────────
    // Governance and ML strategy.
    
    // Admin features require full JSON compression and System Key Shield.
    let backstage_router = Router::new()
        .nest("/pages", Router::new()
            .route("/", post(backstage::orchestration::save_page_layout))
            .route("/active", get(backstage::orchestration::list_active_pages))
            .route("/{slug}", get(backstage::orchestration::get_page_layout).delete(backstage::orchestration::delete_page_layout)))
        
        .nest("/scenarios", Router::new()
            .route("/", post(backstage::strategy::create_scenario).get(backstage::strategy::list_scenarios))
            .route("/{slug}", get(backstage::strategy::get_scenario).put(backstage::strategy::update_scenario).delete(backstage::strategy::delete_scenario))
            .route("/{slug}/reload", post(backstage::strategy::reload_scenario))
            .route("/reload-all", post(backstage::strategy::reload_all_scenarios)))
        
        .nest("/intelligence", Router::new()
            .route("/features/user/{user_id}", get(backstage::intelligence::get_user_features))
            .route("/features/item/{item_id}", get(backstage::intelligence::get_item_features))
            .route("/features/trending", get(backstage::intelligence::get_trending_items))
            .route("/suggestions", get(backstage::intelligence::list_suggestions))
            .route("/suggestions/{id}/approve", post(backstage::intelligence::approve_suggestion))
            .route("/chatbot/ask", post(backstage::intelligence::chatbot_ask)))
        
        .nest("/system", Router::new()
            .route("/reload", post(pulse::metrics::reload_engine_atomic))
            .route("/models/reload", post(pulse::metrics::reload_models))
            .route("/models/stats", get(pulse::metrics::get_model_stats))
            .route("/security/status", get(pulse::metrics::get_security_status))
            .route("/prewarm", post(stage::discovery::prewarm_scenarios)))
        
        // GLOBAL BACKSTAGE SHIELD: Apply admin auth to all admin routes at once
        .layer(from_fn(move |req, next| {
            let engine = engine.clone();
            async move {
                // We use the helper logic directly or call the shared auth middleware
                crate::api::middleware::service::platform_security_middleware(req, next).await
            }
        }))
        .layer(CompressionConfig::new().build());

    // ─── COMPOSE THE SYMPHONY ────────────────────────────────────────────

    Router::new()
        .nest("/recommendation", stage_router.nest("/admin", backstage_router))
        .merge(pulse_router)
}
