//! Scenario management endpoints and handlers.

use axum::{
    extract::{Path, Extension},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;

/// Mount all scenario management routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", post(create_scenario).get(list_scenarios))
        .route("/:slug", get(get_scenario).put(update_scenario).delete(delete_scenario))
        .route("/:slug/reload", post(reload_scenario))
        .route("/reload-all", post(reload_all_scenarios))
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// POST /api/v1/scenarios
async fn create_scenario() -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario creation
    Err(AppError::Internal("Scenario creation not yet implemented".to_string()))
}

/// GET /api/v1/scenarios
async fn list_scenarios() -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario listing
    Err(AppError::Internal("Scenario listing not yet implemented".to_string()))
}

/// GET /api/v1/scenarios/:slug
async fn get_scenario(
    Path(_slug): Path<String>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario retrieval
    Err(AppError::Internal("Scenario retrieval not yet implemented".to_string()))
}

/// PUT /api/v1/scenarios/:slug
async fn update_scenario(
    Path(_slug): Path<String>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario update
    Err(AppError::Internal("Scenario update not yet implemented".to_string()))
}

/// DELETE /api/v1/scenarios/:slug
async fn delete_scenario(
    Path(_slug): Path<String>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario deletion
    Err(AppError::Internal("Scenario deletion not yet implemented".to_string()))
}

/// POST /api/v1/scenarios/:slug/reload
async fn reload_scenario(
    Path(_slug): Path<String>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    // TODO: Implement scenario reload
    Err(AppError::Internal("Scenario reload not yet implemented".to_string()))
}

/// POST /api/v1/scenarios/reload-all
async fn reload_all_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<usize>>, AppError> {
    let count = engine.reload_scenarios().await?;
    info!(scenario_count = count, "All scenarios reloaded successfully");
    Ok(Json(StandardResponse::success(count)))
}
