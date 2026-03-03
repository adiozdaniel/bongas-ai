//! Metrics sub-module for the Symphony Pulse.
//! Handles System stats, cache performance, and ingestion health monitoring.

use axum::{extract::Extension, Json, http::HeaderMap};
use std::sync::Arc;

use crate::engine::BongasEngine;
use crate::api::models::{
    StandardResponse, CacheStatsResponse, KafkaMetricsResponse, KafkaHealthResponse,
    ModelReloadResponse, ModelStatsResponse, SecurityStatusResponse,
};
use crate::error::AppError;
use crate::api::middleware::service::extract_request_id_from_headers;

/// GET /api/v1/recommendation/admin/system/metrics
pub async fn get_resilience_metrics(
    Extension(engine): Extension<Arc<BongasEngine>>,
    headers: HeaderMap,
) -> Json<StandardResponse<crate::resilience::types::RegistrySnapshot>> {
    let request_id = extract_request_id_from_headers(&headers);
    let registry = engine.resilience_metrics.registry();
    let snapshot = registry.snapshot();
    Json(StandardResponse::success(snapshot).with_request_id(request_id))
}

/// GET /api/v1/recommendation/admin/system/cache/stats
pub async fn get_cache_stats(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<CacheStatsResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let stats = engine.get_cache_stats();
    Ok(Json(StandardResponse::success(CacheStatsResponse {
        l1_hits: stats.l1_hits,
        l1_misses: stats.l1_misses,
        l2_hits: stats.l2_hits,
        l2_misses: stats.l2_misses,
        hit_rate: engine.get_hit_rate(),
    }).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/system/ingestion/metrics
pub async fn get_ingestion_metrics(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<KafkaMetricsResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let health = engine.ingestion_health().await;
    Ok(Json(StandardResponse::success(KafkaMetricsResponse {
        metrics: serde_json::to_value(health.sources).unwrap_or_default(),
    }).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/system/ingestion/health
pub async fn get_ingestion_health(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<KafkaHealthResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let health = engine.ingestion_health().await;
    Ok(Json(StandardResponse::success(KafkaHealthResponse {
        healthy: health.healthy,
        total_consumers: health.total_sources,
        unhealthy_consumers: health.degraded_sources,
        total_lag: 0, 
        global_success_rate: if health.total_messages_ingested > 0 {
            (health.total_messages_ingested - health.total_errors) as f64 / health.total_messages_ingested as f64
        } else {
            1.0
        },
    }).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/system/models/reload
pub async fn reload_models(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ModelReloadResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let count = engine.reload_models().await?;
    Ok(Json(StandardResponse::success(ModelReloadResponse {
        model_count: count,
        message: format!("Successfully reloaded {} ONNX models", count),
    }).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/system/models/stats
pub async fn get_model_stats(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<ModelStatsResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let count = engine.model_count().await;
    Ok(Json(StandardResponse::success(ModelStatsResponse {
        loaded_models: count,
    }).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/system/security/status
pub async fn get_security_status(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<SecurityStatusResponse>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let status = engine.get_security_status().await;
    Ok(Json(StandardResponse::success(SecurityStatusResponse {
        validated: status.validated,
        security_enabled: status.security_enabled,
        layers_configured: status.layers_configured,
    }).with_request_id(request_id)))
}

/// POST /api/v1/recommendation/admin/system/reload
/// Performs an atomic refresh of all engine components: Scenarios, Pipelines, and Page Layouts.
pub async fn reload_engine_atomic(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    
    // 1. Reload Scenarios & Rules
    let scenario_count = engine.reload_scenarios().await?;
    
    // 2. Reload Page Layouts & Nav Mesh
    let page_count = engine.pages.load_all_active().await?;
    
    // 3. Reload ML Models
    let model_count = engine.reload_models().await?;

    Ok(Json(StandardResponse::success(serde_json::json!({
        "status": "synchronized",
        "scenarios_reloaded": scenario_count,
        "pages_reloaded": page_count,
        "models_reloaded": model_count
    })).with_request_id(request_id)))
}
