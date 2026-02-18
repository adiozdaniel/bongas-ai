//! Recommendation endpoints and handlers.

use axum::{
    extract::{Path, Extension, Query},
    routing::get,
    Json, Router,
    response::sse::{Event, Sse},
};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::info;

use crate::engine::BongasEngine;
use crate::api::models::{StandardResponse, RecommendationItem, ContextParams};
use crate::api::models::recommendation::FeedRow;
use crate::error::AppError;
use crate::ingestion::types::UserActivity;

/// Mount all recommendation routes.
pub fn routes() -> Router {
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
    context_params: Option<ContextParams>,
    context_data: serde_json::Value,
    offset: usize,
    _limit: usize,
) -> Result<Vec<RecommendationItem>, AppError> {
    let engine_ref = engine.as_ref();
    
    // Extract context parameters
    let (profile_id, maturity_rating, device_type) = if let Some(cp) = context_params {
        (cp.profile_id, cp.maturity_rating, cp.device_type)
    } else {
        (None, None, None)
    };
    
    // 1. Get Scenario display limit
    let display_limit = {
        let scenarios = engine_ref.scenarios.read().await;
        scenarios.get(scenario_slug)
            .map(|s| s.initial_display_limit as usize)
            .unwrap_or(5)
    };

    // 2. Execute First Window (Instant-On)
    let (items, _stats) = engine_ref
        .execute_scenario_with_stats_contextual(
            scenario_slug, 
            user_id, 
            profile_id.clone(),
            maturity_rating.clone(),
            device_type.clone(),
            context_data.clone(), 
            Some(display_limit)
        )
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
        let engine_clone_for_ingestion = engine_ref.ingestion_manager.clone();
        tokio::spawn(async move {
            let manager = engine_clone_for_ingestion.read().await;
            let api_source = manager.api_source();
            for act in activities {
                let _ = api_source.ingest(act).await;
            }
        });

        // ─── Ecosystem Synergy (Phase 14) ──────────────────────────────────
        let engine_clone_for_synergy = engine.clone();
        let uid = uid;
        let pid = profile_id.clone();
        let slug = scenario_slug.to_string();
        let item_ids: Vec<i32> = final_items.iter().map(|i| i.item_id).collect();
        
        tokio::spawn(async move {
            let manager = engine_clone_for_synergy.ingestion_manager.read().await;
            manager.broadcast_recommendations(uid, pid, slug, item_ids).await;
        });
    }

    // 3. ─── Background Pre-Warming (Phase 12) ───────────────────────────
    if offset == 0 {
        let engine_clone_for_warming = engine.clone();
        let slug = scenario_slug.to_string();
        let ctx = context_data.clone();
        
        tokio::spawn(async move {
            // We force a refresh of the cache by executing the scenario with a larger internal limit (None)
            let _ = engine_clone_for_warming.execute_scenario_with_stats(&slug, user_id, ctx, None).await;
        });
    }

    Ok(final_items)
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let engine_clone = engine.clone();
    let profile_id = context_params.profile_id.clone();
    let maturity_rating = context_params.maturity_rating.clone();
    let device_type = context_params.device_type.clone();

    let stream = stream::unfold(
        vec!["continue_watching", "supreme_ranker", "trending_now"],
        move |mut slugs| {
            let engine = engine_clone.clone();
            let profile_id = profile_id.clone();
            let maturity_rating = maturity_rating.clone();
            let device_type = device_type.clone();
            
            async move {
                if slugs.is_empty() {
                    return None;
                }
                let slug = slugs.remove(0);
                
                let cp = ContextParams {
                    profile_id: profile_id.clone(),
                    maturity_rating: maturity_rating.clone(),
                    device_type: device_type.clone(),
                };

                // Execute scenario
                let items_res = execute_and_map(
                    engine,
                    slug,
                    Some(user_id),
                    Some(cp),
                    serde_json::json!({}),
                    0,
                    20,
                ).await;

                if let Ok(items) = items_res {
                    if items.is_empty() && slug == "continue_watching" {
                        // Return empty row for Continue Watching instead of skip to keep client logic simple
                        // or just return the next row.
                        // Let's recurse or just return an empty row.
                    }

                    let row = FeedRow {
                        title: match slug {
                            "continue_watching" => "Continue Watching".to_string(),
                            "supreme_ranker" => "Picked For You".to_string(),
                            _ => "Trending Now".to_string(),
                        },
                        row_type: "horizontal_list".to_string(),
                        scenario: slug.to_string(),
                        items,
                    };

                    let event = Event::default()
                        .json_data(&row)
                        .unwrap_or_else(|_| Event::default().comment("error"));
                    
                    Some((Ok(event), slugs))
                } else {
                    Some((Ok(Event::default().comment("error")), slugs))
                }
            }
        },
    );

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(
        engine.clone(), 
        "continue_watching", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(user_id, scenario = "continue_watching", result_count = items.len(), "Continue watching recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_trending(
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(
        engine.clone(), 
        "trending_now", 
        None, 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(scenario = "trending_now", result_count = items.len(), "Trending recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_because_you_watched(
    Path((user_id, item_id)): Path<(i32, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let context = serde_json::json!({ "item_id": item_id });
    let items = execute_and_map(
        engine.clone(), 
        "because_you_watched", 
        Some(user_id), 
        Some(context_params),
        context, 
        0, 
        usize::MAX
    ).await?;

    info!(user_id, item_id, scenario = "because_you_watched", result_count = items.len(), "Because you watched recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_genre_recommendations(
    Path((genre, user_id)): Path<(String, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let scenario_slug = format!("genre_{}", genre.to_lowercase());
    let items = execute_and_map(
        engine.clone(), 
        &scenario_slug, 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(user_id, genre = genre, scenario = scenario_slug, result_count = items.len(), "Genre recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_new_releases(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(
        engine.clone(), 
        "new_releases", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(user_id, scenario = "new_releases", result_count = items.len(), "New releases recommendations served");
    Ok(Json(StandardResponse::success(items)))
}

async fn get_live_tv(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let items = execute_and_map(
        engine.clone(), 
        "live_tv", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(user_id, scenario = "live_tv", result_count = items.len(), "Live TV recommendations served");
    Ok(Json(StandardResponse::success(items)))
}
