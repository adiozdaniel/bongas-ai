use axum::extract::{Extension, Path, Json, Query};
use std::sync::Arc;
use tracing::{info, error};
use serde::Deserialize;

use crate::engine::BongasEngine;
use crate::api::models::ApiResponse;

/// GET /api/v1/features/user/:user_id
/// Get user features from feature repository
pub async fn get_user_features(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(user_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, String> {
    info!(user_id = user_id, "Fetching user features");

    match engine.feature_repo().get_user_features(user_id).await {
        Ok(Some(features)) => {
            let response = serde_json::json!({
                "user_id": user_id,
                "features": features,
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Ok(None) => Err(format!("User {} not found", user_id)),
        Err(e) => {
            error!(error = %e, "Failed to fetch user features");
            Err(format!("Failed to fetch features: {}", e))
        }
    }
}

/// GET /api/v1/features/item/:item_id
/// Get item features from feature repository
pub async fn get_item_features(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(item_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, String> {
    info!(item_id = item_id, "Fetching item features");

    match engine.feature_repo().get_item_features(item_id).await {
        Ok(Some(features)) => {
            let response = serde_json::json!({
                "item_id": item_id,
                "features": features,
            });
            Ok(Json(ApiResponse::success(response)))
        }
        Ok(None) => Err(format!("Item {} not found", item_id)),
        Err(e) => {
            error!(error = %e, "Failed to fetch item features");
            Err(format!("Failed to fetch features: {}", e))
        }
    }
}

/// GET /api/v1/features/trending
/// Get trending items from feature repository
pub async fn get_trending_items(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Query(params): Query<TrendingQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, String> {
    let limit = params.limit.unwrap_or(10);
    info!(limit = limit, "Fetching trending items");

    match engine.feature_repo().get_trending_items(limit).await {
        Ok(items) => {
            let response: Vec<serde_json::Value> = items
                .into_iter()
                .map(|item| serde_json::json!({
                    "item_id": item.item_id,
                    "title": item.title,
                    "genres": item.genres,
                    "view_count": item.view_count,
                    "like_count": item.like_count,
                    "completion_rate": item.completion_rate,
                }))
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch trending items");
            Err(format!("Failed to fetch trending: {}", e))
        }
    }
}

#[derive(Deserialize)]
pub struct TrendingQuery {
    pub limit: Option<i64>,
}

