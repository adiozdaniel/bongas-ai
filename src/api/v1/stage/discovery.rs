//! Discovery sub-module for the Symphony Stage.
//! Handles Genesis entry point and Page Orchestration (SSE).

use axum::{
    extract::{Path, Extension, Query},
    response::sse::{Event, Sse, KeepAlive},
    body::Body,
    http::{Request, HeaderMap},
    Json,
};
use std::time::Duration;
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::warn;

use crate::engine::BongasEngine;
use crate::api::models::{ContextParams, StandardResponse};
use crate::api::models::recommendation::{FeedRow, SymphonyNavigation, RecommendationItem};
use crate::api::middleware::service::{extract_request_id, extract_request_id_from_headers};
use crate::api::middleware::identity::IdentityContext;
use crate::pages::types::PageCompositionItem;
use crate::api::v1::stage::service::execute_and_map;

// ─── Genesis Orchestrator ──────────────────────────────────────────────────

/// GET /api/v1/recommendation
/// The primary initiation call. Resolves entry point and navigation mesh.
pub async fn genesis(
    Query(mut context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = extract_request_id(&req);
    let identity = req.extensions().get::<IdentityContext>().cloned();
    
    if let Some(ref id) = identity {
        context_params.merge_identity(id);
    }

    let user_id = context_params.user_id;
    let engine_clone = engine.clone();
    let cp_base = context_params.clone();
    let rid = request_id.clone();
    let identity_key = cp_base.visitor_id.as_deref().map(|s| s.to_string());

    // 1. Resolve Navigation Mesh (Personalized)
    let nav_mesh = engine.pages.get_nav_mesh_contextual(identity_key.as_deref()).await;
    
    // 2. Resolve Landing Page
    let landing_layout = engine.pages.get_landing_page_contextual(
        cp_base.device_type.as_deref(),
        cp_base.maturity_rating.as_deref(),
        identity_key.as_deref()
    ).await.unwrap_or(None);

    let (composition, landing_slug) = match landing_layout {
        Some(l) => (l.composition, l.page_slug.0),
        None => {
            warn!(request_id = %rid, "No landing page found, using emergency fallback");
            (vec![
                PageCompositionItem { slug: "trending_now".into(), fallback_slug: None, row_type: "hero_carousel".into(), row_style: Some("promotional".into()) },
            ], "home".to_string())
        }
    };

    // Slice for first batch (Genesis always starts at 0)
    let batch_size = 5;
    let initial_batch: Vec<_> = composition.iter().take(batch_size).cloned().collect();
    let total_count = composition.len();

    // 3. Assemble Genesis Stream
    let nav_event = Event::default()
        .event("navigation")
        .json_data(nav_mesh.main.iter().map(|p| SymphonyNavigation {
            slug: p.page_slug.0.clone(),
            title: p.page_slug.0.replace('_', " ").to_uppercase(),
            nav_type: "main".to_string(),
        }).collect::<Vec<_>>())
        .unwrap_or_else(|_| Event::default().comment("nav_error"));

    let sub_nav_event = Event::default()
        .event("sub_navigation")
        .json_data(nav_mesh.sub.iter().map(|p| SymphonyNavigation {
            slug: p.page_slug.0.clone(),
            title: p.page_slug.0.replace('_', " "),
            nav_type: "sub".to_string(),
        }).collect::<Vec<_>>())
        .unwrap_or_else(|_| Event::default().comment("sub_nav_error"));

    let manifest_event = Event::default()
        .event("manifest")
        .json_data(serde_json::json!({ 
            "expected_rows": total_count,
            "request_id": rid.clone(),
            "page": landing_slug,
            "prewarm_scenarios": composition.iter().skip(batch_size).map(|c| &c.slug).collect::<Vec<_>>()
        }))
        .unwrap_or_else(|_| Event::default().comment("manifest_error"));

    let initial_stream = stream::iter(vec![Ok(nav_event), Ok(sub_nav_event), Ok(manifest_event)]);

    let scenario_stream = stream::iter(initial_batch)
        .map(move |comp_item| {
            let engine = engine_clone.clone();
            let cp = cp_base.clone();
            let rid_inner = rid.clone();
            let uid = user_id;
            async move {
                execute_row(engine, cp, rid_inner, comp_item, uid).await
            }
        })
        .buffered(5);

    // 4. Continuation Logic
    let mut continuation_stream = vec![];
    if total_count > batch_size {
        let cont_event = Event::default()
            .event("continuation")
            .json_data(serde_json::json!({
                "next_url": format!("/api/v1/recommendation/page/{}?offset={}&batch={}", landing_slug, batch_size, batch_size)
            }))
            .unwrap_or_else(|_| Event::default().comment("cont_error"));
        continuation_stream.push(Ok(cont_event));
    }

    let full_stream = initial_stream.chain(scenario_stream).chain(stream::iter(continuation_stream));

    Sse::new(full_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// ─── Page Orchestrator ─────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct PageParams {
    pub offset: Option<usize>,
    pub batch: Option<usize>,
}

/// GET /api/v1/recommendation/page/{slug}
/// Fetches a specific batch of rows for a page.
pub async fn get_page_recommendations(
    Path(page_slug): Path<String>,
    Query(page_params): Query<PageParams>,
    Query(mut context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = extract_request_id(&req);
    let identity = req.extensions().get::<IdentityContext>().cloned();
    
    if let Some(ref id) = identity {
        context_params.merge_identity(id);
    }

    let user_id = context_params.user_id;
    let engine_clone = engine.clone();
    let cp_base = context_params.clone();
    let rid = request_id.clone();
    let identity_key = cp_base.visitor_id.as_deref().map(|s| s.to_string());

    let offset = page_params.offset.unwrap_or(0);
    let batch_size = page_params.batch.unwrap_or(5);

    // 1. Resolve Layout
    let layout_res = engine.pages.get_layout_contextual(
        &page_slug, 
        cp_base.device_type.as_deref(), 
        cp_base.maturity_rating.as_deref(),
        identity_key.as_deref()
    ).await;
    
    let composition = match layout_res {
        Ok(Some(layout)) => layout.composition,
        _ => vec![]
    };

    let total_count = composition.len();
    let batch_items: Vec<_> = composition.iter().skip(offset).take(batch_size).cloned().collect();

    // 2. Initial Manifest
    let manifest_event = Event::default()
        .event("manifest")
        .json_data(serde_json::json!({ 
            "offset": offset,
            "batch_size": batch_items.len(),
            "total_rows": total_count,
            "request_id": rid.clone()
        }))
        .unwrap_or_else(|_| Event::default().comment("manifest_error"));

    let initial_stream = stream::iter(vec![Ok(manifest_event)]);

    // 3. Scenario Stream
    let scenario_stream = stream::iter(batch_items)
        .map(move |comp_item| {
            let engine = engine_clone.clone();
            let cp = cp_base.clone();
            let rid_inner = rid.clone();
            let uid = user_id;
            async move {
                execute_row(engine, cp, rid_inner, comp_item, uid).await
            }
        })
        .buffered(5);

    // 4. Continuation
    let mut continuation_stream = vec![];
    if offset + batch_size < total_count {
        let cont_event = Event::default()
            .event("continuation")
            .json_data(serde_json::json!({
                "next_url": format!("/api/v1/recommendation/page/{}?offset={}&batch={}", page_slug, offset + batch_size, batch_size)
            }))
            .unwrap_or_else(|_| Event::default().comment("cont_error"));
        continuation_stream.push(Ok(cont_event));
    }

    let full_stream = initial_stream.chain(scenario_stream).chain(stream::iter(continuation_stream));

    Sse::new(full_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// ─── Scenario Detail (See All) ─────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct ScenarioParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/recommendation/scenario/{slug}
pub async fn get_scenario_detail(
    Path(slug): Path<String>,
    Query(params): Query<ScenarioParams>,
    Query(context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    headers: HeaderMap,
) -> Result<Json<StandardResponse<Vec<RecommendationItem>>>, crate::error::AppError> {
    let request_id = extract_request_id_from_headers(&headers);
    
    let items = execute_and_map(
        engine,
        &slug,
        context_params.user_id, 
        Some(context_params),
        serde_json::json!({}),
        params.offset.unwrap_or(0),
        params.limit.unwrap_or(20),
        request_id.clone(),
    ).await?;

    Ok(Json(StandardResponse::success(items).with_request_id(request_id)))
}

// ─── Predictive Pre-warming ───────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct PrewarmRequest {
    pub slugs: Vec<String>,
    pub user_id: i32,
}

/// POST /api/v1/recommendation/admin/system/prewarm
pub async fn prewarm_scenarios(
    Extension(engine): Extension<Arc<BongasEngine>>,
    headers: HeaderMap,
    Json(payload): Json<PrewarmRequest>,
) -> Json<StandardResponse<()>> {
    let request_id = extract_request_id_from_headers(&headers);
    
    for slug in payload.slugs {
        let engine = engine.clone();
        let uid = payload.user_id;
        let rid = request_id.clone();
        
        tokio::spawn(async move {
            let _ = execute_and_map(
                engine,
                &slug,
                Some(uid),
                None,
                serde_json::json!({}),
                0,
                50,
                rid,
            ).await;
        });
    }

    Json(StandardResponse::success(()).with_request_id(request_id))
}

// ─── Private Helpers ───────────────────────────────────────────────────────

async fn execute_row(
    engine: Arc<BongasEngine>,
    cp: ContextParams,
    rid: String,
    comp_item: PageCompositionItem,
    user_id: Option<i32>,
) -> Result<Event, Infallible> {
    let slug = comp_item.slug.clone();
    let fallback = comp_item.fallback_slug.clone();
    
    // Safety check
    let is_safe = {
        let s_map = engine.scenarios.scenarios.read().await;
        s_map.get(&slug).map(|s| {
            if s.maturity_rating == "18" {
                cp.maturity_rating.as_deref() == Some("18")
            } else { true }
        }).unwrap_or(true)
    };

    if !is_safe {
        return Ok(Event::default().comment(format!("safety: restricted:{}", slug)));
    }

    let mut result: Result<Vec<RecommendationItem>, crate::error::AppError> = execute_and_map(
        engine.clone(),
        &slug,
        user_id,
        Some(cp.clone()),
        serde_json::json!({}),
        0,
        20,
        rid.clone(),
    ).await;

    if result.is_err() {
        if let Some(f_slug) = fallback {
            result = execute_and_map(
                engine.clone(),
                &f_slug,
                user_id,
                Some(cp),
                serde_json::json!({}),
                0,
                20,
                rid.clone(),
            ).await;
        }
    }

    match result {
        Ok(items) => {
            let title = {
                let s_map = engine.scenarios.scenarios.read().await;
                s_map.get(&slug).map(|s| s.name.clone()).unwrap_or_else(|| slug.replace('_', " "))
            };
            let row = FeedRow {
                title,
                row_type: comp_item.row_type,
                row_style: comp_item.row_style,
                scenario: slug,
                items,
            };
            Ok(Event::default().event("row").json_data(&row).unwrap_or_else(|_| Event::default().comment("serial_error")))
        }
        Err(_) => Ok(Event::default().comment(format!("error: failed:{}", slug)))
    }
}
