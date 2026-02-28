use axum::{
    extract::{Path, Extension, Query},
    Json,
    response::sse::{Event, Sse, KeepAlive},
    body::Body,
    http::Request,
};
use std::time::Duration;
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{info, warn};

use crate::engine::BongasEngine;
use crate::api::models::{StandardResponse, RecommendationItem, ContextParams};
use crate::api::models::recommendation::FeedRow;
use crate::error::AppError;
use crate::api::middleware::service::extract_request_id;
use super::service::execute_and_map;

pub async fn get_page_recommendations(
    Path((page_slug, user_id)): Path<(String, i32)>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = extract_request_id(&req);
    let engine_clone = engine.clone();
    let profile_id = context_params.profile_id.clone();
    let maturity_rating = context_params.maturity_rating.clone();
    let device_type = context_params.device_type.clone();

    // 1. Fetch active pages for the navigation context
    let active_pages = engine.pages.list_active_pages().await.unwrap_or_default();
    
    // 2. Fetch layout for the requested page
    let layout_res = engine.pages.get_layout(&page_slug).await;
    let scenario_slugs = match layout_res {
        Ok(Some(layout)) => layout.scenario_slugs,
        _ => {
            warn!(request_id = %request_id, page = %page_slug, "Page layout not found, stream will only contain navigation");
            vec![]
        }
    };

    let stream = stream::unfold(
        (true, active_pages, scenario_slugs), // State: (should_send_nav, navigation, remaining_slugs)
        move |(send_nav, nav, mut slugs)| {
            let engine = engine_clone.clone();
            let profile_id = profile_id.clone();
            let maturity_rating = maturity_rating.clone();
            let device_type = device_type.clone();
            let request_id_inner = request_id.clone();
            
            async move {
                // First event: Send navigation metadata
                if send_nav {
                    let event = Event::default()
                        .event("navigation")
                        .json_data(serde_json::json!({ "active_pages": nav }))
                        .unwrap_or_else(|_| Event::default().comment("nav_serialization_error"));
                    return Some((Ok(event), (false, vec![], slugs)));
                }

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
                    engine.clone(),
                    &slug,
                    Some(user_id),
                    Some(cp),
                    serde_json::json!({}),
                    0,
                    20,
                ).await;

                match items_res {
                    Ok(items) => {
                        let title = {
                            let scenarios = engine.scenarios.scenarios.read().await;
                            scenarios.get(&slug)
                                .map(|s| s.name.clone())
                                .unwrap_or_else(|| slug.replace('_', " "))
                        };

                        let row = FeedRow {
                            title,
                            row_type: "horizontal_list".to_string(),
                            scenario: slug.to_string(),
                            items,
                        };

                        let event = Event::default()
                            .event("row")
                            .json_data(&row)
                            .unwrap_or_else(|_| Event::default().comment("serialization_error"));
                        
                        Some((Ok(event), (false, vec![], slugs)))
                    }
                    Err(e) => {
                        warn!(request_id = %request_id_inner, scenario = %slug, error = ?e, "Scenario failed in SSE, skipping");
                        let error_msg = format!("error: scenario '{}' failed", slug);
                        Some((Ok(Event::default().comment(error_msg)), (false, vec![], slugs)))
                    }
                }
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

pub async fn get_home_recommendations(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = extract_request_id(&req);
    let engine_clone = engine.clone();
    let profile_id = context_params.profile_id.clone();
    let maturity_rating = context_params.maturity_rating.clone();
    let device_type = context_params.device_type.clone();

    // 1. Fetch dynamic layout from the Pages Module
    let layout_res = engine.pages.get_layout("home").await;
    let scenario_slugs = match layout_res {
        Ok(Some(layout)) => layout.scenario_slugs,
        _ => {
            warn!(request_id = %request_id, "Home layout not found in Pages module, using defaults");
            vec!["trending_now".to_string(), "personalized_picks".to_string(), "home_feed".to_string()]
        }
    };

    let stream = stream::unfold(
        scenario_slugs,
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
                    engine.clone(),
                    &slug,
                    Some(user_id),
                    Some(cp),
                    serde_json::json!({}),
                    0,
                    20,
                ).await;

                if let Ok(items) = items_res {
                    // Try to get scenario name for the title
                    let title = {
                        let scenarios = engine.scenarios.scenarios.read().await;
                        scenarios.get(&slug)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| slug.replace('_', " "))
                    };

                    let row = FeedRow {
                        title,
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

pub async fn get_continue_watching(
    Path(user_id): Path<i32>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
    req: Request<Body>,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, AppError> {
    let request_id = extract_request_id(&req);
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
