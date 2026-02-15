//! Feature repository endpoints and handlers.

use axum::{
    extract::{Extension, Path, Json, Query},
    routing::get,
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use tracing::{info, error};

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::error::{AppError, ScenarioError, CacheError};

/// Mount all feature routes.
pub fn routes() -> Router {
    Router::new()
        .route("/user/:user_id", get(get_user_features))
        .route("/item/:item_id", get(get_item_features))
        .route("/trending", get(get_trending_items))
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// GET /api/v1/features/user/:user_id
async fn get_user_features(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(user_id): Path<i32>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    info!(user_id = user_id, "Fetching user features");

    match engine.feature_repo().get_user_features(user_id).await {
        Ok(Some(features)) => {
            let response = serde_json::json!({
                "user_id": user_id,
                "features": features,
            });
            Ok(Json(StandardResponse::success(response)))
        }
        Ok(None) => Err(AppError::Scenario(ScenarioError::NotFound(format!("User {}", user_id)))),
        Err(e) => {
            error!(error = %e, "Failed to fetch user features");
            Err(AppError::Cache(CacheError::Operation(format!("Failed to fetch features: {}", e))))
        }
    }
}

/// GET /api/v1/features/item/:item_id
async fn get_item_features(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Path(item_id): Path<i32>,
) -> Result<Json<StandardResponse<serde_json::Value>>, AppError> {
    info!(item_id = item_id, "Fetching item features");

    match engine.feature_repo().get_item_features(item_id).await {
        Ok(Some(features)) => {
            let response = serde_json::json!({
                "item_id": item_id,
                "features": features,
            });
            Ok(Json(StandardResponse::success(response)))
        }
        Ok(None) => Err(AppError::Scenario(ScenarioError::NotFound(format!("Item {}", item_id)))),
        Err(e) => {
            error!(error = %e, "Failed to fetch item features");
            Err(AppError::Cache(CacheError::Operation(format!("Failed to fetch features: {}", e))))
        }
    }
}

/// GET /api/v1/features/trending
async fn get_trending_items(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Query(params): Query<TrendingQuery>,
) -> Result<Json<StandardResponse<Vec<serde_json::Value>>>, AppError> {
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
            Ok(Json(StandardResponse::success(response)))
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch trending items");
            Err(AppError::Cache(CacheError::Operation(format!("Failed to fetch trending: {}", e))))
        }
    }
}

#[derive(Deserialize)]
struct TrendingQuery {
    pub limit: Option<i64>,
}
