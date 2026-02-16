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

use crate::db::models::ScenarioConfig;

/// Mount all scenario management routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_scenarios))
        .route("/:slug", get(get_scenario))
        .route("/reload-all", post(reload_all_scenarios))
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// GET /api/v1/scenarios
async fn list_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<ScenarioConfig>>>, AppError> {
    let configs = engine.scenario_factory.repo().find_all_enabled().await?;
    Ok(Json(StandardResponse::success(configs)))
}

/// GET /api/v1/scenarios/:slug
async fn get_scenario(
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ScenarioConfig>>, AppError> {
    let config = engine.scenario_factory.repo().find_by_slug(&slug).await?
        .ok_or_else(|| AppError::NotFound(format!("Scenario {} not found", slug)))?;
    Ok(Json(StandardResponse::success(config)))
}

/// POST /api/v1/scenarios/reload-all
async fn reload_all_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<usize>>, AppError> {
    let count = engine.reload_scenarios().await?;
    info!(scenario_count = count, "All scenarios reloaded successfully");
    Ok(Json(StandardResponse::success(count)))
}
