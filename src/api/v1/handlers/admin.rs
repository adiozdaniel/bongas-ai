use axum::{
    extract::Extension,
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::engine::BongasEngine;
use crate::api::models::{
    ApiResponse, CacheStatsResponse, InvalidateCacheRequest, HealthResponse,
};
use crate::api::error::ApiError;
use crate::api::error::ApiResult;

/// GET /api/v1/admin/cache-stats
pub async fn get_cache_stats(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<CacheStatsResponse>>> {
    let stats = engine.get_cache_stats();

    let response = CacheStatsResponse {
        l1_hits: stats.l1_hits,
        l1_misses: stats.l1_misses,
        l2_hits: stats.l2_hits,
        l2_misses: stats.l2_misses,
        hit_rate: stats.overall_hit_rate,
    };

    info!(
        l1_hits = stats.l1_hits,
        l1_misses = stats.l1_misses,
        l2_hits = stats.l2_hits,
        l2_misses = stats.l2_misses,
        hit_rate = stats.overall_hit_rate,
        "Cache stats retrieved"
    );

    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/admin/cache/invalidate
pub async fn invalidate_cache(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(req): Json<InvalidateCacheRequest>,
) -> ApiResult<Json<ApiResponse<bool>>> {
    match (req.scenario_slug, req.profile_id) {
        (Some(slug), Some(profile_id)) => {
            // TODO: Implement specific scenario + profile invalidation
            // For now, return a placeholder error
            Err(ApiError::Internal("Specific cache invalidation not yet implemented".to_string()))
        }
        (None, Some(profile_id)) => {
            // TODO: Implement all scenarios for profile invalidation
            // For now, return a placeholder error
            Err(ApiError::Internal("Profile cache invalidation not yet implemented".to_string()))
        }
        _ => {
            Err(ApiError::BadRequest("Invalid invalidation request".to_string()))
        }
    }
}

/// GET /health
pub async fn health_check() -> ApiResult<Json<ApiResponse<HealthResponse>>> {
    let response = HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        uptime_seconds: 0, // TODO: Implement uptime tracking
    };

    info!("Health check requested");

    Ok(Json(ApiResponse::success(response)))
}