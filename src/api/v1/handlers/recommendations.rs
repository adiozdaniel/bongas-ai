use axum::{
    extract::{Path, Query, Extension},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use serde::Deserialize;
use tracing::{info, error};

use crate::engine::BongasEngine;
use crate::api::models::{ApiResponse, RecommendationItem, PaginationParams};
use crate::api::error::ApiError;
use crate::api::error::ApiResult;

/// GET /api/v1/recommendations/home/:user_id
pub async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Query(params): Query<PaginationParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "personalized_home";

    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, Some(user_id), serde_json::json!({}))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get home recommendations: {}", e)))?;

    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let response_items: Vec<RecommendationItem> = items
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
        .collect();

    info!(
        user_id = user_id,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Home recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/continue-watching/:user_id
pub async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "continue_watching";

    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, Some(user_id), serde_json::json!({}))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get continue watching: {}", e)))?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        user_id = user_id,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Continue watching recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/trending
pub async fn get_trending(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "trending_now";

    // Trending is user-agnostic, use placeholder user_id
    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, None, serde_json::json!({}))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get trending: {}", e)))?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Trending recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/because-you-watched/:user_id/:item_id
pub async fn get_because_you_watched(
    Path((user_id, item_id)): Path<(i32, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "because_you_watched";

    // Pass item_id as context
    let context = serde_json::json!({
        "item_id": item_id
    });

    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, Some(user_id), context)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get because you watched: {}", e)))?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        user_id = user_id,
        item_id = item_id,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Because you watched recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/genre/:genre/:user_id
pub async fn get_genre_recommendations(
    Path((genre, user_id)): Path<(String, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = format!("genre_{}", genre.to_lowercase());

    let (items, _stats) = engine
        .execute_scenario_with_stats(&scenario_slug, Some(user_id), serde_json::json!({}))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::ScenarioNotFound { slug: scenario_slug.clone() }
            } else {
                ApiError::Internal(format!("Failed to get genre recommendations: {}", e))
            }
        })?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        user_id = user_id,
        genre = genre,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Genre recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/new-releases/:user_id
pub async fn get_new_releases(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "new_releases";

    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, Some(user_id), serde_json::json!({}))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get new releases: {}", e)))?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        user_id = user_id,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "New releases recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}

/// GET /api/v1/recommendations/live-tv/:user_id
pub async fn get_live_tv(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> ApiResult<Json<ApiResponse<Vec<RecommendationItem>>>> {
    let scenario_slug = "live_tv";

    let (items, _stats) = engine
        .execute_scenario_with_stats(scenario_slug, Some(user_id), serde_json::json!({}))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get live TV: {}", e)))?;

    let response_items: Vec<RecommendationItem> = items
        .into_iter()
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
            rank: (idx + 1) as i32,
            metadata: item.metadata.clone(),
        })
        .collect();

    info!(
        user_id = user_id,
        scenario = scenario_slug,
        result_count = response_items.len(),
        "Live TV recommendations served"
    );

    Ok(Json(ApiResponse::success(response_items)))
}