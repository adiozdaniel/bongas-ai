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
use crate::api::models::scenario::{CreateScenarioRequest, UpdateScenarioRequest};

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
async fn create_scenario(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(req): Json<CreateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioConfig>>, AppError> {
    let slug = req.slug.clone();
    let config = engine.scenario_factory.repo().create(req).await?;
    
    // Hot-reload the new scenario
    engine.reload_scenario(&slug).await
        .map_err(|e| AppError::Internal(format!("Failed to reload created scenario: {}", e)))?;

    info!(slug = %slug, "Scenario created and reloaded");
    Ok(Json(StandardResponse::success(config)))
}

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

/// PUT /api/v1/scenarios/:slug
async fn update_scenario(
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(req): Json<UpdateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioConfig>>, AppError> {
    let config = engine.scenario_factory.repo().update(&slug, req).await?;
    
    // Hot-reload the updated scenario
    engine.reload_scenario(&slug).await
        .map_err(|e| AppError::Internal(format!("Failed to reload updated scenario: {}", e)))?;

    info!(slug = %slug, "Scenario updated and reloaded");
    Ok(Json(StandardResponse::success(config)))
}

/// DELETE /api/v1/scenarios/:slug
async fn delete_scenario(
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    engine.scenario_factory.repo().delete(&slug).await?;
    
    // Remove from engine's active scenarios
    engine.remove_scenario(&slug).await;

    info!(slug = %slug, "Scenario deleted");
    Ok(Json(StandardResponse::success(())))
}

/// POST /api/v1/scenarios/:slug/reload
async fn reload_scenario(
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<bool>>, AppError> {
    let success = engine.reload_scenario(&slug).await?;
    Ok(Json(StandardResponse::success(success)))
}

/// POST /api/v1/scenarios/reload-all
async fn reload_all_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<usize>>, AppError> {
    let count = engine.reload_scenarios().await?;
    info!(scenario_count = count, "All scenarios reloaded successfully");
    Ok(Json(StandardResponse::success(count)))
}
