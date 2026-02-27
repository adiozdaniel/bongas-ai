//! Scenario management endpoints and handlers.

use axum::{
    extract::{Path, Extension},
    routing::{get, post},
    Json, Router,
    http::HeaderMap,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;

use crate::db::models::ScenarioWithStrategy;
use crate::api::models::scenario::{CreateScenarioRequest, UpdateScenarioRequest};
use tower_http::request_id::RequestId;

/// Mount all scenario management routes.
pub fn routes() -> Router {
    Router::new()
        .route("/", post(create_scenario).get(list_scenarios))
        .route("/{slug}", get(get_scenario).put(update_scenario).delete(delete_scenario))
        .route("/{slug}/reload", post(reload_scenario))
        .route("/reload-all", post(reload_all_scenarios))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn authorize_admin(headers: &HeaderMap, engine: &BongasEngine) -> Result<(), AppError> {
    let system_key = &engine.config.security.system_api_key;
    
    let provided_key = headers.get("X-Platform-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing X-Platform-Key for admin access".to_string()))?;

    if provided_key != system_key {
        return Err(AppError::Unauthorized("Invalid administrative key".to_string()));
    }

    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// POST /api/v1/scenarios
async fn create_scenario(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<CreateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let slug = payload.slug.clone();
    let config = engine.scenario_factory().repo().create(payload).await?;
    
    // Hot-reload the new scenario
    engine.reload_scenario(&slug).await
        .map_err(|e| AppError::Internal(format!("Failed to reload created scenario: {}", e)))?;

    info!(request_id = %request_id, slug = %slug, "Scenario created and reloaded");
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// GET /api/v1/scenarios
async fn list_scenarios(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<ScenarioWithStrategy>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let configs = engine.scenario_factory().repo().find_all_active().await?;
    Ok(Json(StandardResponse::success(configs).with_request_id(request_id)))
}

/// GET /api/v1/scenarios/:slug
async fn get_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let config = engine.scenario_factory().repo().find_by_slug(&slug).await?
        .ok_or_else(|| AppError::NotFound(format!("Scenario {} not found", slug)))?;
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// PUT /api/v1/scenarios/:slug
async fn update_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<UpdateScenarioRequest>,
) -> Result<Json<StandardResponse<ScenarioWithStrategy>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let config = engine.scenario_factory().repo().update(&slug, payload).await?;
    
    // Hot-reload the updated scenario
    engine.reload_scenario(&slug).await
        .map_err(|e| AppError::Internal(format!("Failed to reload updated scenario: {}", e)))?;

    info!(request_id = %request_id, slug = %slug, "Scenario updated and reloaded");
    Ok(Json(StandardResponse::success(config).with_request_id(request_id)))
}

/// DELETE /api/v1/scenarios/:slug
async fn delete_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<()>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    engine.scenario_factory().repo().delete(&slug).await?;
    
    // Remove from engine's active scenarios
    engine.remove_scenario(&slug).await;

    info!(request_id = %request_id, slug = %slug, "Scenario deleted");
    Ok(Json(StandardResponse::success(()).with_request_id(request_id)))
}

/// POST /api/v1/scenarios/:slug/reload
async fn reload_scenario(
    headers: HeaderMap,
    Path(slug): Path<String>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<bool>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let success = engine.reload_scenario(&slug).await?;
    Ok(Json(StandardResponse::success(success).with_request_id(request_id)))
}

/// POST /api/v1/scenarios/reload-all
async fn reload_all_scenarios(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<usize>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let count = engine.reload_scenarios().await?;
    info!(request_id = %request_id, scenario_count = count, "All scenarios reloaded successfully");
    Ok(Json(StandardResponse::success(count).with_request_id(request_id)))
}
