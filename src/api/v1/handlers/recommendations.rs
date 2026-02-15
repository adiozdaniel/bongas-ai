use axum::{
    extract::{Path, Query, Extension},
    Json,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::{StandardResponse, RecommendationItem, PaginationParams};
use crate::error::AppError;

/// Shared helper: execute a scenario and map engine items to API RecommendationItems.
async fn execute_and_map(
    engine: &BongasEngine,
    scenario_slug: &str,
    user_id: Option<i32>,
    context: serde_json::Value,
    offset: usize,
    limit: usize,
) -> Result<Vec<RecommendationItem>, AppError> {
    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, user_id, context)
        .await?;

    Ok(items
        .into_iter()
        .skip(offset)
        .take(limit)
        .enumerate()
        .map(|(idx, item)| RecommendationItem {
            item_id: item.item_id,
            title: item.metadata.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            thumbnail_url: item.metadata.get("thumbnail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            score: item.score,
            rank: (offset + idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect())
}

/// GET /api/v1/recommendations/home/:user_id
pub async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Query(params): Query<PaginationParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let items = execute_and_map(&engine, "personalized_home", Some(user_id), serde_json::json!({}), offset, limit).await?;

    info!(user_id, scenario = "personalized_home", result_count = items.len(), "Home recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/continue-watching/:user_id
pub async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(&engine, "continue_watching", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "continue_watching", result_count = items.len(), "Continue watching recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/trending
pub async fn get_trending(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(&engine, "trending_now", None, serde_json::json!({}), 0, usize::MAX).await?;

    info!(scenario = "trending_now", result_count = items.len(), "Trending recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/because-you-watched/:user_id/:item_id
pub async fn get_because_you_watched(
    Path((user_id, item_id)): Path<(i32, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let context = serde_json::json!({ "item_id": item_id });
    let items = execute_and_map(&engine, "because_you_watched", Some(user_id), context, 0, usize::MAX).await?;

    info!(user_id, item_id, scenario = "because_you_watched", result_count = items.len(), "Because you watched recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/genre/:genre/:user_id
pub async fn get_genre_recommendations(
    Path((genre, user_id)): Path<(String, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let scenario_slug = format!("genre_{}", genre.to_lowercase());
    let items = execute_and_map(&engine, &scenario_slug, Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, genre = genre, scenario = scenario_slug, result_count = items.len(), "Genre recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/new-releases/:user_id
pub async fn get_new_releases(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(&engine, "new_releases", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "new_releases", result_count = items.len(), "New releases recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

/// GET /api/v1/recommendations/live-tv/:user_id
pub async fn get_live_tv(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(&engine, "live_tv", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "live_tv", result_count = items.len(), "Live TV recommendations served");
    Ok(Json(StandardResponse::success(items)))
}
