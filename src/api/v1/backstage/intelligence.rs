//! Intelligence sub-module for the Symphony Backstage.
//! Handles Feature Store inspection, ML Suggestions, and LLM Chatbot interface.

use axum::{
    extract::{Path, Extension, Query},
    Json,
    http::HeaderMap,
};
use std::sync::Arc;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::AppError;
use crate::api::middleware::service::extract_request_id_from_headers;

// ─── Feature Store Inspection ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct TrendingQuery {
    pub category: Option<String>,
    pub limit: Option<usize>,
}

/// GET /api/v1/recommendation/admin/features/user/:user_id
pub async fn get_user_features(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(user_id): Path<i32>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let features = engine.execution.item_feature_service.get_user_features(user_id).await?;
    Ok(Json(StandardResponse::success(serde_json::to_value(features).unwrap_or_default()).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/features/item/:item_id
pub async fn get_item_features(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(item_id): Path<i32>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let features = engine.execution.item_feature_service.get_item_features_batch(&[item_id]).await?;
    Ok(Json(StandardResponse::success(serde_json::to_value(features).unwrap_or_default()).with_request_id(request_id)))
}

/// GET /api/v1/recommendation/admin/features/trending
pub async fn get_trending_items(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Query(params): Query<TrendingQuery>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let items = engine.execution.item_feature_service.get_popular_content(
        0, 
        params.limit.unwrap_or(10) as i64
    ).await?;
    Ok(Json(StandardResponse::success(serde_json::to_value(items).unwrap_or_default()).with_request_id(request_id)))
}

// ─── ML Suggestions & LLM ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct ChatbotQuery {
    pub message: String,
}

pub async fn chatbot_ask(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<ChatbotQuery>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
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
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let impact = engine.simulate_suggestion(id).await?;
    Ok(Json(StandardResponse::success(impact).with_request_id(request_id)))
}

pub async fn list_suggestions(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<serde_json::Value>>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    let suggestions = engine.list_suggestions().await?;
    Ok(Json(StandardResponse::success(suggestions).with_request_id(request_id)))
}

pub async fn approve_suggestion(
    headers: HeaderMap,
    Path(id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    engine.approve_suggestion(id).await?;
    Ok(Json(StandardResponse::success(serde_json::json!({ "message": "Suggestion approved and rule activated" })).with_request_id(request_id)))
}

pub async fn reject_suggestion(
    headers: HeaderMap,
    Path(id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    engine.reject_suggestion(id).await?;
    Ok(Json(StandardResponse::success(serde_json::json!({ "message": "Suggestion rejected" })).with_request_id(request_id)))
}
