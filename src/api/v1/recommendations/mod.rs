//! Recommendation endpoints and handlers.

pub mod handlers;
pub mod service;

use axum::{
    routing::{get, post},
    Router,
};
use self::handlers::*;

/// Mount all public recommendation routes.
/// The Stage: Focused on frictionless content delivery and feedback.
pub fn routes() -> Router {
    Router::new()
        // 1. The Genesis Entry Point: Resolves World & Nav Mesh
        .route("/", get(genesis))
        
        // 2. The Page Orchestrator: Contextual View Delivery
        .route("/page/{slug}", get(get_page_recommendations))
        
        // 3. Scenario Detail: Deep Pagination (See All)
        .route("/scenario/{slug}", get(get_scenario_detail))
        
        // 4. Ingestion: Real-time behavior tracking
        .route("/ingest", post(ingest_activities))
}
