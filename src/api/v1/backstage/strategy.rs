//! Strategy sub-module for the Symphony Backstage.
//! Handles Scenario and Pipeline CRUD, along with hot-reloading logic.

use axum::{
    extract::{Path, Extension},
    Json,
    http::HeaderMap,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;
use crate::db::models::ScenarioWithStrategy;
use crate::api::models::scenario::{CreateScenarioRequest, UpdateScenarioRequest};
use crate::api::middleware::service::extract_request_id_from_headers;

/// POST /api/v1/recommendation/admin/scenarios
pub async fn create_scenario(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<CreateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let slug = payload.slug.clone();
    let config = engine.scenario_factory().repo().create(payload).await?;
    
    // Hot-reload the new scenario
    engine.reload_scenario(&slug).await?;

    info!(request_id = %request_id, slug = %slug, "Scenario created and reloaded");
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/scenarios
pub async fn list_scenarios(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<ScenarioWithStrategy>>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let configs = engine.scenario_factory().repo().find_all_active().await?;
    Ok(Json(StandardResponse::success(configs).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/scenarios/:slug
pub async fn get_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let config = engine.scenario_factory().repo().find_by_slug(&slug).await?
        .ok_or_else(|| AppError::NotFound(format!("Scenario {} not found", slug)))?;
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// PUT /api/v1/recommendation/admin/scenarios/:slug
pub async fn update_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<UpdateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let config = engine.scenario_factory().repo().update(&slug, payload).await?;
    
    // Hot-reload the updated scenario
    engine.reload_scenario(&slug).await?;

    info!(request_id = %request_id, slug = %slug, "Scenario updated and reloaded");
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// DELETE /api/v1/recommendation/admin/scenarios/:slug
pub async fn delete_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    engine.scenario_factory().repo().delete(&slug).await?;
    
    // Remove from engine's active scenarios
    engine.remove_scenario(&slug).await;

    info!(request_id = %request_id, slug = %slug, "Scenario deleted");
    Ok(Json(StandardResponse::success(()).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/scenarios/:slug/reload
pub async fn reload_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<bool>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let success = engine.reload_scenario(&slug).await?;
    Ok(Json(StandardResponse::success(success).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/scenarios/reload-all
pub async fn reload_all_scenarios(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<usize>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let count = engine.reload_scenarios().await?;
    info!(request_id = %request_id, scenario_count = count, "All scenarios reloaded successfully");
    Ok(Json(StandardResponse::success(count).with_request_id(request_id)))
}
