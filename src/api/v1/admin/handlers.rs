use axum::{
    extract::{Extension, Path},
    Json,
    http::HeaderMap,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::{
    StandardResponse, CacheStatsResponse, KafkaMetricsResponse, KafkaHealthResponse,
    ModelReloadResponse, ModelStatsResponse, SecurityStatusResponse,
};
use crate::error::AppError;
use tower_http::request_id::RequestId;

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

/// POST /api/v1/recommendation/admin/system/reload
/// Performs an atomic refresh of all engine components: Scenarios, Pipelines, and Page Layouts.
pub async fn reload_engine_atomic(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    
    info!(request_id = %request_id, "Triggering atomic symphony engine refresh...");
    
    // 1. Reload Scenarios & Rules
    let scenario_count = engine.reload_scenarios().await?;
    
    // 2. Reload Page Layouts & Nav Mesh
    let page_count = engine.pages.load_all_active().await?;
    
    // 3. Reload ML Models
    let model_count = engine.reload_models().await?;

    info!(
        request_id = %request_id, 
        scenarios = scenario_count, 
        pages = page_count, 
        models = model_count, 
        "Atomic engine refresh complete"
    );

    Ok(Json(StandardResponse::success(serde_json::json!({
        "status": "synchronized",
        "scenarios_reloaded": scenario_count,
        "pages_reloaded": page_count,
        "models_reloaded": model_count
    })).with_request_id(request_id)))
}

#[derive(serde::Deserialize)]
pub struct ChatbotQuery {
    pub message: String,
}

pub async fn chatbot_ask(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<ChatbotQuery>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let suggestion_id = engine.chatbot_process_query(&payload.message).await?;
    Ok(Json(StandardResponse::success(serde_json::json!({ 
        "suggestion_id": suggestion_id,
        "message": "I've analyzed your request and created a rule suggestion. You can now simulate it or approve it." 
    })).with_request_id(request_id)))
}

pub async fn simulate_suggestion(
    headers: HeaderMap,
    Path(id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let impact = engine.simulate_suggestion(id).await?;
    Ok(Json(StandardResponse::success(impact).with_request_id(request_id)))
}

pub async fn list_suggestions(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<serde_json::Value>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let suggestions = engine.list_suggestions().await?;
    Ok(Json(StandardResponse::success(suggestions).with_request_id(request_id)))
}

pub async fn approve_suggestion(
    headers: HeaderMap,
    Path(id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    engine.approve_suggestion(id).await?;
    Ok(Json(StandardResponse::success(serde_json::json!({ "message": "Suggestion approved and rule activated" })).with_request_id(request_id)))
}

pub async fn reject_suggestion(
    headers: HeaderMap,
    Path(id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    engine.reject_suggestion(id).await?;
    Ok(Json(StandardResponse::success(serde_json::json!({ "message": "Suggestion rejected" })).with_request_id(request_id)))
}

pub async fn get_resilience_metrics(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Json<StandardResponse<crate::resilience::types::RegistrySnapshot>> {
    let registry = engine.resilience_metrics.registry();
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    
    let snapshot = registry.snapshot();
    Json(StandardResponse::success(snapshot).with_request_id(request_id))
}

pub async fn get_cache_stats(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<CacheStatsResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let stats = engine.get_cache_stats();
    Ok(Json(StandardResponse::success(CacheStatsResponse {
        l1_hits: stats.l1_hits,
        l1_misses: stats.l1_misses,
        l2_hits: stats.l2_hits,
        l2_misses: stats.l2_misses,
        hit_rate: engine.get_hit_rate(),
    }).with_request_id(request_id)))
}

pub async fn get_ingestion_metrics(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<KafkaMetricsResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let health = engine.ingestion_health().await;
    Ok(Json(StandardResponse::success(KafkaMetricsResponse {
        metrics: serde_json::to_value(health.sources).unwrap_or_default(),
    }).with_request_id(request_id)))
}

pub async fn get_ingestion_health(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<KafkaHealthResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
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

pub async fn reload_models(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<ModelReloadResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let count = engine.reload_models().await?;
    Ok(Json(StandardResponse::success(ModelReloadResponse {
        model_count: count,
        message: format!("Successfully reloaded {} ONNX models", count),
    }).with_request_id(request_id)))
}

pub async fn get_model_stats(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<ModelStatsResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let count = engine.model_count().await;
    Ok(Json(StandardResponse::success(ModelStatsResponse {
        loaded_models: count,
    }).with_request_id(request_id)))
}

pub async fn get_security_status(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<SecurityStatusResponse>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    authorize_admin(&headers, &engine)?;
    let status = engine.get_security_status().await;
    Ok(Json(StandardResponse::success(SecurityStatusResponse {
        validated: status.validated,
        security_enabled: status.security_enabled,
        layers_configured: status.layers_configured,
    }).with_request_id(request_id)))
}
