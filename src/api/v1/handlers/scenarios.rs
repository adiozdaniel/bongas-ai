use axum::{
    extract::{Path, Extension},
    Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::engine::BongasEngine;
use crate::api::models::ApiResponse;
use crate::api::error::ApiError;
use crate::api::error::ApiResult;

/// POST /api/v1/scenarios - Not implemented yet
pub async fn create_scenario() -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario creation not yet implemented".to_string()))
}

/// GET /api/v1/scenarios - Not implemented yet
pub async fn list_scenarios() -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario listing not yet implemented".to_string()))
}

/// GET /api/v1/scenarios/:slug - Not implemented yet
pub async fn get_scenario(
    Path(_slug): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario retrieval not yet implemented".to_string()))
}

/// PUT /api/v1/scenarios/:slug - Not implemented yet
pub async fn update_scenario(
    Path(_slug): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario update not yet implemented".to_string()))
}

/// DELETE /api/v1/scenarios/:slug - Not implemented yet
pub async fn delete_scenario(
    Path(_slug): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario deletion not yet implemented".to_string()))
}

/// POST /api/v1/scenarios/:slug/reload - Not implemented yet
pub async fn reload_scenario(
    Path(_slug): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Scenario reload not yet implemented".to_string()))
}

/// POST /api/v1/scenarios/reload-all
pub async fn reload_all_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<usize>>> {
    match engine.reload_scenarios().await {
        Ok(count) => {
            info!(scenario_count = count, "All scenarios reloaded successfully");
            Ok(Json(ApiResponse::success(count)))
        }
        Err(e) => {
            error!(error = ?e, "Failed to reload all scenarios");
            Err(ApiError::Internal(format!("Failed to reload scenarios: {}", e)))
        }
    }
}