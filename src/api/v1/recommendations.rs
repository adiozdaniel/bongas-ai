//! Recommendation endpoints and handlers.

use axum::{
    extract::{Path, Extension},
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::{StandardResponse, RecommendationItem};
use crate::api::models::recommendation::{HomeFeedResponse, FeedRow};
use crate::error::AppError;
use crate::ingestion::types::UserActivity;

/// Mount all recommendation routes.
pub fn routes() -> Router {
// ... (rest of routes)
    Router::new()
        .route("/home/:user_id", get(get_home_recommendations))
        .route("/continue-watching/:user_id", get(get_continue_watching))
        .route("/trending", get(get_trending))
        .route("/because-you-watched/:user_id/:item_id", get(get_because_you_watched))
        .route("/genre/:genre/:user_id", get(get_genre_recommendations))
        .route("/new-releases/:user_id", get(get_new_releases))
        .route("/live-tv/:user_id", get(get_live_tv))
}

// ─── Shared Helper ──────────────────────────────────────────────────────────

/// Execute a scenario and map engine items to API RecommendationItems.
async fn execute_and_map(
    engine: Arc<BongasEngine>,
    scenario_slug: &str,
    user_id: Option<i32>,
    context: serde_json::Value,
    offset: usize,
    _limit: usize,
) -> Result<Vec<RecommendationItem>, AppError> {
    let engine_ref = engine.as_ref();
    
    // 1. Get Scenario display limit
    let display_limit = {
        let scenarios = engine_ref.scenarios.read().await;
        scenarios.get(scenario_slug)
            .map(|s| s.initial_display_limit as usize)
            .unwrap_or(5)
    };

    // 2. Execute First Window (Instant-On)
    let (items, _stats) = engine_ref
        .execute_scenario_with_stats(scenario_slug, user_id, context.clone(), Some(display_limit))
        .await?;

    let final_items: Vec<RecommendationItem> = items
        .into_iter()
        .skip(offset)
        .take(display_limit)
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

    // ─── Impression Tracking (Baze-Style) ──────────────────────────────
    if let Some(uid) = user_id {
        let activities: Vec<UserActivity> = final_items.iter().map(|item| {
            UserActivity::Impression {
                user_id: uid,
                item_id: item.item_id,
                scenario_slug: Some(scenario_slug.to_string()),
                timestamp: chrono::Utc::now(),
            }
        }).collect();

        // Ingest activities asynchronously
        let engine_clone = engine_ref.ingestion_manager.clone();
        tokio::spawn(async move {
            let manager = engine_clone.read().await;
            let api_source = manager.api_source();
            for act in activities {
                let _ = api_source.ingest(act).await;
            }
        });

        // ─── Ecosystem Synergy (Phase 14) ──────────────────────────────────
        let engine_clone = engine.clone();
        let uid = uid;
        let slug = scenario_slug.to_string();
        let item_ids: Vec<i32> = final_items.iter().map(|i| i.item_id).collect();
        
        tokio::spawn(async move {
            let manager = engine_clone.ingestion_manager.read().await;
            manager.broadcast_recommendations(uid, slug, item_ids).await;
        });
    }

    // 3. ─── Background Pre-Warming (Phase 12) ───────────────────────────
    if offset == 0 {
        let engine_clone = engine.clone();
        let slug = scenario_slug.to_string();
        let ctx = context.clone();
        
        tokio::spawn(async move {
            // We force a refresh of the cache by executing the scenario with a larger internal limit (None)
            // Note: The execute_scenario logic already handles saving to StagingManager.
            let _ = engine_clone.execute_scenario_with_stats(&slug, user_id, ctx, None).await;
        });
    }

    Ok(final_items)
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<HomeFeedResponse>>, AppError> {
    // Concurrently fetch multiple scenarios for the home feed
    let (continue_watching, supreme_ranker, trending) = tokio::join!(
        execute_and_map(engine.clone(), "continue_watching", Some(user_id), serde_json::json!({}), 0, 10),
        execute_and_map(engine.clone(), "supreme_ranker", Some(user_id), serde_json::json!({}), 0, 20),
        execute_and_map(engine.clone(), "trending_now", None, serde_json::json!({}), 0, 20),
    );

    let mut rows = Vec::new();

    // 1. Continue Watching (if items exist)
    if let Ok(items) = continue_watching {
        if !items.is_empty() {
            rows.push(FeedRow {
                title: "Continue Watching".to_string(),
                row_type: "horizontal_list".to_string(),
                scenario: "continue_watching".to_string(),
                items,
            });
        }
    }

    // 2. Supreme Ranker (Grok-style personalized feed)
    if let Ok(items) = supreme_ranker {
        rows.push(FeedRow {
            title: "Picked For You".to_string(),
            row_type: "horizontal_list".to_string(),
            scenario: "supreme_ranker".to_string(),
            items,
        });
    }

    // 3. Trending Now
    if let Ok(items) = trending {
        rows.push(FeedRow {
            title: "Trending Now".to_string(),
            row_type: "horizontal_list".to_string(),
            scenario: "trending_now".to_string(),
            items,
        });
    }

    info!(user_id, row_count = rows.len(), "Master Home Feed assembled");
    
    Ok(Json(StandardResponse::success(HomeFeedResponse {
        rows,
        experiment_id: None,
    })))
}

async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(engine.clone(), "continue_watching", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "continue_watching", result_count = items.len(), "Continue watching recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_trending(
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(engine.clone(), "trending_now", None, serde_json::json!({}), 0, usize::MAX).await?;

    info!(scenario = "trending_now", result_count = items.len(), "Trending recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_because_you_watched(
    Path((user_id, item_id)): Path<(i32, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let context = serde_json::json!({ "item_id": item_id });
    let items = execute_and_map(engine.clone(), "because_you_watched", Some(user_id), context, 0, usize::MAX).await?;

    info!(user_id, item_id, scenario = "because_you_watched", result_count = items.len(), "Because you watched recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_genre_recommendations(
    Path((genre, user_id)): Path<(String, i32)>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let scenario_slug = format!("genre_{}", genre.to_lowercase());
    let items = execute_and_map(engine.clone(), &scenario_slug, Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, genre = genre, scenario = scenario_slug, result_count = items.len(), "Genre recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_new_releases(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(engine.clone(), "new_releases", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "new_releases", result_count = items.len(), "New releases recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_live_tv(
    Path(user_id): Path<i32>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(engine.clone(), "live_tv", Some(user_id), serde_json::json!({}), 0, usize::MAX).await?;

    info!(user_id, scenario = "live_tv", result_count = items.len(), "Live TV recommendations served");
    Ok(Json(StandardResponse::success(items)))
}
