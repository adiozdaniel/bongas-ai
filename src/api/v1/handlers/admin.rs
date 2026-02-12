use axum::{
    extract::Extension,
    Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::engine::BongasEngine;
use crate::api::models::{
    ApiResponse, HealthResponse,
    KafkaMetricsResponse, KafkaHealthResponse, CacheStatsResponse, ModelReloadResponse, ModelStatsResponse,
    SecurityStatusResponse,
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

/// POST /api/v1/admin/cache/invalidate - Not implemented yet
pub async fn invalidate_cache() -> ApiResult<Json<ApiResponse<()>>> {
    Err(ApiError::Internal("Cache invalidation not yet implemented".to_string()))
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

/// GET /api/v1/admin/kafka/metrics
pub async fn get_kafka_metrics(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<KafkaMetricsResponse>>> {
    let metrics = engine.kafka_metrics().get_all_metrics().await;

    info!("Kafka metrics retrieved");

    Ok(Json(ApiResponse::success(KafkaMetricsResponse { metrics })))
}

/// GET /api/v1/admin/kafka/health
pub async fn get_kafka_health(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<KafkaHealthResponse>>> {
    let health = engine.kafka_health().await;

    let response = KafkaHealthResponse {
        healthy: health.healthy,
        total_consumers: health.total_consumers,
        unhealthy_consumers: health.unhealthy_consumers,
        total_lag: health.total_lag,
        global_success_rate: health.global_success_rate,
    };

    info!(
        healthy = health.healthy,
        consumers = health.total_consumers,
        "Kafka health check"
    );

    Ok(Json(ApiResponse::success(response)))
}

/// POST /api/v1/admin/models/reload
/// Hot-reload all ONNX models without server restart
pub async fn reload_models(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<ModelReloadResponse>>> {
    info!("ONNX model hot-reload requested");

    match engine.reload_models().await {
        Ok(count) => {
            info!(model_count = count, "ONNX models reloaded successfully");
            Ok(Json(ApiResponse::success(ModelReloadResponse {
                model_count: count,
                message: format!("Successfully reloaded {} ONNX models", count),
            })))
        }
        Err(e) => {
            error!(error = %e, "Failed to reload ONNX models");
            Err(ApiError::Internal(format!("Failed to reload models: {}", e)))
        }
    }
}

/// GET /api/v1/admin/models/stats
/// Get statistics about loaded ONNX models
pub async fn get_model_stats(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<ModelStatsResponse>>> {
    let count = engine.model_count().await;

    info!(loaded_models = count, "Model stats retrieved");

    Ok(Json(ApiResponse::success(ModelStatsResponse {
        loaded_models: count,
    })))
}

/// GET /api/v1/admin/security/status
/// Get current security validation status
pub async fn get_security_status(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<SecurityStatusResponse>>> {
    let status = engine.get_security_status().await;

    info!(
        validated = status.validated,
        security_enabled = status.security_enabled,
        layers = status.layers_configured,
        "Security status retrieved"
    );

    Ok(Json(ApiResponse::success(SecurityStatusResponse {
        validated: status.validated,
        security_enabled: status.security_enabled,
        layers_configured: status.layers_configured,
    })))
}
