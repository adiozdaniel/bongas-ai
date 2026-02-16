use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::engine::BongasEngine;
use crate::api::models::{
    StandardResponse, CacheStatsResponse, KafkaMetricsResponse, KafkaHealthResponse,
    ModelReloadResponse, ModelStatsResponse, SecurityStatusResponse,
};
use crate::error::AppError;

/// Mount all admin routes.
pub fn routes() -> Router {
    Router::new()
        .route("/cache/stats", get(get_cache_stats))
        .route("/ingestion/metrics", get(get_ingestion_metrics))
        .route("/ingestion/health", get(get_ingestion_health))
        .route("/models/reload", post(reload_models))
        .route("/models/stats", get(get_model_stats))
        .route("/security/status", get(get_security_status))
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn get_cache_stats(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<CacheStatsResponse>>, AppError> {
    let stats = engine.get_cache_stats();
    Ok(Json(StandardResponse::success(CacheStatsResponse {
        l1_hits: stats.l1_hits,
        l1_misses: stats.l1_misses,
        l2_hits: stats.l2_hits,
        l2_misses: stats.l2_misses,
        hit_rate: engine.get_cache_hit_rate(),
    })))
}

async fn get_ingestion_metrics(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<KafkaMetricsResponse>>, AppError> {
    let health = engine.ingestion_health().await;
    // Simplification for the response model
    Ok(Json(StandardResponse::success(KafkaMetricsResponse {
        metrics: serde_json::to_value(health.sources).unwrap_or_default(),
    })))
}

async fn get_ingestion_health(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<KafkaHealthResponse>>, AppError> {
    let health = engine.ingestion_health().await;
    Ok(Json(StandardResponse::success(KafkaHealthResponse {
        healthy: health.healthy,
        total_consumers: health.total_sources,
        unhealthy_consumers: health.degraded_sources,
        total_lag: 0, // Not explicitly tracked in IngestionHealth currently
        global_success_rate: if health.total_messages_ingested > 0 {
            (health.total_messages_ingested - health.total_errors) as f64 / health.total_messages_ingested as f64
        } else {
            1.0
        },
    })))
}

async fn reload_models(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ModelReloadResponse>>, AppError> {
    let count = engine.reload_models().await?;
    Ok(Json(StandardResponse::success(ModelReloadResponse {
        model_count: count,
        message: format!("Successfully reloaded {} ONNX models", count),
    })))
}

async fn get_model_stats(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ModelStatsResponse>>, AppError> {
    let count = engine.model_count().await;
    Ok(Json(StandardResponse::success(ModelStatsResponse {
        loaded_models: count,
    })))
}

async fn get_security_status(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<SecurityStatusResponse>>, AppError> {
    let status = engine.get_security_status().await;
    Ok(Json(StandardResponse::success(SecurityStatusResponse {
        validated: status.validated,
        security_enabled: status.security_enabled,
        layers_configured: status.layers_configured,
    })))
}
