//! V1 API Router assembly.
//! The Grand Composer: Context-Aware Routing for the Bongas-AI Symphony.

use axum::{
    routing::{get, post},
    Router,
    middleware::from_fn,
};
use std::sync::Arc;
use crate::middlewares::{
    platform_security::system_security_middleware,
};
use crate::config::{AppConfig, CompressionConfig};
use crate::engine::coordination::service::BongasEngine;
use crate::api::v1::{stage, backstage, pulse};

/// Build the complete v1 API router with context-aware infrastructure logic.
pub fn routes(_engine: Arc<BongasEngine>, _config: Arc<AppConfig>) -> Router {
    // ─── THE STAGE: Public Discovery Pillar ───────────────────────────────
    // Optimized for SSE and high-throughput discovery streams.
    let stage_router = Router::new()
        .route("/", get(stage::discovery::genesis))
        .route("/page", get(stage::discovery::get_page_recommendations))
        .route("/scenario/{slug}", get(stage::discovery::get_scenario_recommendations))
        .route("/ingest", post(stage::ingestion::ingest_activity))
        .route("/ingest/batch", post(stage::ingestion::ingest_activities));

    // ─── THE BACKSTAGE: Administrative Pillar ─────────────────────────────
    // Shielded with System-Key Authorization and heavy compression for data.
    let backstage_router = Router::new()
        // Orchestration (Pages & Layouts)
        .route("/pages/active", get(backstage::orchestration::list_active_pages))
        .route("/pages/{slug}", get(backstage::orchestration::get_page_layout))
        .route("/pages", post(backstage::orchestration::save_page_layout))
        .route("/pages/{slug}", axum::routing::delete(backstage::orchestration::delete_page_layout))
        
        // Strategy (Scenarios & Reloading)
        .route("/scenarios", post(backstage::strategy::create_scenario))
        .route("/scenarios/active", get(backstage::strategy::list_active_scenarios))
        .route("/scenarios/{slug}", get(backstage::strategy::get_scenario_config))
        .route("/scenarios/{slug}", axum::routing::patch(backstage::strategy::update_scenario))
        .route("/scenarios/{slug}", axum::routing::delete(backstage::strategy::delete_scenario))
        .route("/reload", post(backstage::strategy::reload_all_scenarios))
        
        // Intelligence (ML Suggestions & Chat)
        .route("/features/user/{user_id}", get(backstage::intelligence::get_user_features))
        .route("/features/item/{item_id}", get(backstage::intelligence::get_item_features))
        .route("/features/trending", get(backstage::intelligence::get_trending_items))
        .route("/suggestions", get(backstage::intelligence::list_suggestions))
        .route("/suggestions/{id}/approve", post(backstage::intelligence::approve_suggestion))
        .route("/suggestions/{id}/simulate", post(backstage::intelligence::simulate_suggestion))
        .route("/suggestions/{id}", axum::routing::delete(backstage::intelligence::reject_suggestion))
        .route("/chat", post(backstage::intelligence::ai_chat_process))
        
        // GLOBAL BACKSTAGE SHIELD: Apply strict system auth to all admin routes
        .layer(from_fn(system_security_middleware))
        .layer(CompressionConfig::new().build());

    // ─── THE PULSE: Operational Pillar ───────────────────────────────────
    // Fast paths for health probes and real-time observability.
    let pulse_router = Router::new()
        .route("/health", get(pulse::health::liveness_check))
        .route("/ready", get(pulse::health::readiness_check))
        .nest("/metrics", Router::new()
            .route("/system", get(pulse::metrics::get_system_stats))
            .route("/cache", get(pulse::metrics::get_cache_performance))
            .route("/ingestion", get(pulse::metrics::get_ingestion_health))
            .route("/kafka", get(pulse::metrics::get_kafka_metrics))
            .route("/reload-atomic", post(pulse::metrics::reload_engine_atomic))
            .route("/models/reload", post(pulse::metrics::reload_models))
            .route("/models/stats", get(pulse::metrics::get_model_stats))
            .route("/security/status", get(pulse::metrics::get_security_status)));

    // ─── COMPOSE THE SYMPHONY ────────────────────────────────────────────

    Router::new()
        .nest("/recommendation", stage_router.nest("/admin", backstage_router))
        .merge(pulse_router)
}
