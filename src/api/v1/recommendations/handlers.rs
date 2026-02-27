use axum::{
    extract::{Path, Extension, Query},
    Json,
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
use super::service::execute_and_map;
use tower_http::request_id::RequestId;

pub async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
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
            let _request_id = request_id.clone();
            
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

                    // Note: SSE events don't naturally fit the StandardResponse envelope per item,
                    // but we ensure the metadata is available if we were to wrap the entire row.
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

pub async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let items = execute_and_map(
        engine.clone(), 
        "continue_watching", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(request_id = %request_id, user_id, scenario = "continue_watching", result_count = items.len(), "Continue watching recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_trending(
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let items = execute_and_map(
        engine.clone(), 
        "trending_now", 
        None, 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(request_id = %request_id, scenario = "trending_now", result_count = items.len(), "Trending recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_because_you_watched(
    Path((user_id, item_id)): Path<(i32, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
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

    info!(request_id = %request_id, user_id, item_id, scenario = "because_you_watched", result_count = items.len(), "Because you watched recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_genre_recommendations(
    Path((genre, user_id)): Path<(String, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
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

    info!(request_id = %request_id, user_id, genre = genre, scenario = scenario_slug, result_count = items.len(), "Genre recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_new_releases(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let items = execute_and_map(
        engine.clone(), 
        "new_releases", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(request_id = %request_id, user_id, scenario = "new_releases", result_count = items.len(), "New releases recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_live_tv(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let items = execute_and_map(
        engine.clone(), 
        "live_tv", 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(request_id = %request_id, user_id, scenario = "live_tv", result_count = items.len(), "Live TV recommendations served");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

pub async fn get_recommendations(
    Path((scenario_slug, user_id)): Path<(String, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = request_id.header_value().to_str().unwrap_or("unknown").to_string();
    let items = execute_and_map(
        engine.clone(), 
        &scenario_slug, 
        Some(user_id), 
        Some(context_params),
        serde_json::json!({}), 
        0, 
        usize::MAX
    ).await?;

    info!(request_id = %request_id, user_id, scenario = scenario_slug, result_count = items.len(), "Recommendations served via generic endpoint");
    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}
