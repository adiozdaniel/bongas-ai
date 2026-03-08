//! Discovery sub-module for the Symphony Stage.
//! Handles Genesis entry point and Page Orchestration (SSE).

use axum::{
    extract::{Path, Extension, Query},
    response::sse::{Event, Sse, KeepAlive},
    body::Body,
};
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{warn, Instrument};

use crate::engine::coordination::service::{BongasEngine, ScenarioDefinition};
use crate::api::{ContextParams, StandardResponse};
use crate::api::{FeedRow, SymphonyNavigation, RecommendationItem, DiscoveryManifest, ContinuationEvent};
use crate::api::extract_request_id;
use crate::api::IdentityContext;
use crate::engine::governance::orchestration::types::models::{PageCompositionItem, PageLayout};
use crate::api::v1::stage::service::execute_and_map;

use axum::extract::Request;

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

    // 0. Fetch Device Configuration
    let config = engine.governance.get_discovery_config(cp_base.device_type.as_deref()).await;
    let batch_size = config.initial_batch_size as usize;

    // 1. Resolve Navigation Mesh (Personalized)
    let nav_mesh = engine.governance.orchestration.get_nav_mesh_contextual(identity_key.as_deref()).await;
    
    let main_nav: Vec<SymphonyNavigation> = nav_mesh.iter()
        .filter(|n| n.nav_type == "main")
        .cloned()
        .collect();

    let sub_nav: Vec<SymphonyNavigation> = nav_mesh.into_iter()
        .filter(|n| n.nav_type == "sub")
        .collect();

    // 2. Resolve Landing Page
    let landing_layout: Option<PageLayout> = engine.governance.orchestration.get_landing_page_contextual(
        cp_base.device_type.as_deref(),
        cp_base.maturity_rating.as_deref(),
        identity_key.as_deref()
    ).await.unwrap_or_else(|_| None);

    let (composition, landing_slug) = match landing_layout {
        Some(l) => (l.composition, l.page_slug.0),
        None => {
            warn!(request_id = %rid, "No landing page found, using emergency fallback");
            (vec![
                PageCompositionItem { slug: "trending_now".into(), fallback_slug: None, row_type: "hero_carousel".into(), row_style: Some("promotional".into()) },
            ], "home".to_string())
        }
    };

    // Slice for first batch
    let initial_batch: Vec<PageCompositionItem> = composition.iter().take(batch_size).cloned().collect();
    let total_count = composition.len();

    // Trigger Ghost Pre-warm for the NEXT batch
    if total_count > batch_size {
        engine.clone().ghost_prewarm(
            user_id,
            landing_slug.clone(),
            batch_size,
            config.continuation_batch_size as usize,
            config.ghost_ttl_seconds as usize,
            serde_json::to_value(&cp_base).unwrap_or_default(),
        );
    }

    // 3. Assemble Genesis Stream
    let nav_event = Event::default()
        .event("navigation")
        .json_data(&SymphonyNavigation {
            slug: landing_slug.clone(),
            title: "Bongas Discovery".to_string(),
            nav_type: "main".to_string(),
            nav_mesh: main_nav,
            landing_slug: landing_slug.clone(),
            total_rows: total_count,
            request_id: rid.clone(),
        })
        .unwrap_or_else(|_| Event::default().comment("serial_error"));

    let sub_nav_event = Event::default()
        .event("sub_navigation")
        .json_data(&sub_nav)
        .unwrap_or_else(|_| Event::default().comment("serial_error"));

    let manifest_event = Event::default()
        .event("manifest")
        .json_data(&DiscoveryManifest {
            total_rows: total_count,
            batch_size: config.initial_batch_size,
            prewarming_active: true,
            request_id: rid.clone(),
        })
        .unwrap_or_else(|_| Event::default().comment("serial_error"));

    // Prepare continuation link
    let mut end_events = Vec::new();
    if total_count > batch_size {
        let continuation = ContinuationEvent {
            next_url: format!("/api/v1/recommendation/page/{}?offset={}", landing_slug, batch_size),
            next_offset: batch_size,
            next_batch: config.continuation_batch_size,
        };
        end_events.push(Ok(Event::default().event("continuation").json_data(&continuation).unwrap_or_else(|_| Event::default().comment("serial_error"))));
    }

    // Spawn concurrent stream
    let stream = stream::iter(initial_batch)
        .map(move |item| {
            let engine = engine_clone.clone();
            let cp = cp_base.clone();
            let rid = rid.clone();
            let slug = item.slug.clone();
            async move {
                execute_row(engine, cp, rid, item, user_id).await
            }
            .instrument(tracing::info_span!("execute_row", %slug))
        })
        .buffered(5);

    let full_stream = stream::iter(vec![Ok(nav_event), Ok(sub_nav_event), Ok(manifest_event)])
        .chain(stream)
        .chain(stream::iter(end_events));

    Sse::new(full_stream.boxed()).keep_alive(KeepAlive::default())
}

// ─── Page Orchestrator (Paginated) ─────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct PageParams {
    pub offset: Option<usize>,
    pub batch: Option<usize>,
}

/// GET /api/v1/recommendation/page/{slug}
/// Streams a specific batch of rows for a given page.
pub async fn get_page_recommendations(
    Path(slug): Path<String>,
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
    let page_slug = slug;
    let cp_base = context_params.clone();
    let rid = request_id.clone();
    let identity_key = cp_base.visitor_id.as_deref().map(|s| s.to_string());

    // 0. Fetch Device Configuration
    let config = engine.governance.get_discovery_config(cp_base.device_type.as_deref()).await;
    let offset = page_params.offset.unwrap_or(0);
    let batch_size = page_params.batch.unwrap_or(config.continuation_batch_size as usize);
    let cache_key = format!("ghost:user_{:?}:page_{}:offset_{}", user_id, page_slug, offset);

    // 1. Try Ghost Cache first
    if let Ok(Some(results)) = engine.cache.get::<Vec<serde_json::Value>>(&cache_key, "ghost_cache", user_id, cp_base.profile_id.as_deref()).await {
        if !results.is_empty() {
            let total_rows_est = offset + batch_size + 10; // Estimated
            let manifest = DiscoveryManifest {
                total_rows: total_rows_est,
                batch_size: batch_size as i32,
                prewarming_active: true,
                request_id: rid.clone(),
            };
            
            let continuation = ContinuationEvent {
                next_url: format!("/api/v1/recommendation/page/{}?offset={}", page_slug, offset + batch_size),
                next_offset: offset + batch_size,
                next_batch: config.continuation_batch_size,
            };

            // Trigger NEXT ghost pre-warm immediately
            engine.clone().ghost_prewarm(
                user_id,
                page_slug.clone(),
                offset + batch_size,
                config.continuation_batch_size as usize,
                config.ghost_ttl_seconds as usize,
                serde_json::to_value(&cp_base).unwrap_or_default(),
            );

            let stream = stream::iter(results).map(|r| {
                Ok(Event::default().event("row").json_data(&r).unwrap_or_else(|_| Event::default().comment("serial_error")))
            });

            let full_stream = stream::once(async move { Ok(Event::default().event("manifest").json_data(&manifest).unwrap_or_else(|_| Event::default().comment("serial_error"))) })
                .chain(stream)
                .chain(stream::once(async move { Ok(Event::default().event("continuation").json_data(&continuation).unwrap_or_else(|_| Event::default().comment("serial_error"))) }));

            return Sse::new(full_stream.boxed()).keep_alive(KeepAlive::default());
        }
    }

    // 2. Fallback to Live Execution
    let layout_res = engine.governance.orchestration.get_layout_contextual(
        &page_slug, 
        cp_base.device_type.as_deref(), 
        cp_base.maturity_rating.as_deref(),
        identity_key.as_deref()
    ).await;
    
    let composition: Vec<PageCompositionItem> = match layout_res {
        Ok(Some(layout)) => layout.composition,
        _ => vec![]
    };
    let total_count = composition.len();

    // Trigger NEXT ghost pre-warm
    if total_count > offset + batch_size {
        engine.clone().ghost_prewarm(
            user_id,
            page_slug.clone(),
            offset + batch_size,
            config.continuation_batch_size as usize,
            config.ghost_ttl_seconds as usize,
            serde_json::to_value(&cp_base).unwrap_or_default(),
        );
    }

    // Slice batch
    let batch: Vec<PageCompositionItem> = composition.into_iter()
        .skip(offset)
        .take(batch_size)
        .collect();

    let manifest_event = Event::default()
        .event("manifest")
        .json_data(&DiscoveryManifest {
            total_rows: total_count,
            batch_size: batch_size as i32,
            prewarming_active: true,
            request_id: rid.clone(),
        })
        .unwrap_or_else(|_| Event::default().comment("serial_error"));

    // Prepare continuation link
    let mut end_events = Vec::new();
    if total_count > offset + batch_size {
        let continuation = ContinuationEvent {
            next_url: format!("/api/v1/recommendation/page/{}?offset={}", page_slug, offset + batch_size),
            next_offset: offset + batch_size,
            next_batch: config.continuation_batch_size,
        };
        end_events.push(Ok(Event::default().event("continuation").json_data(&continuation).unwrap_or_else(|_| Event::default().comment("serial_error"))));
    }

    // 2. Stream Batch
    let stream = stream::iter(batch)
        .map(move |item| {
            let engine = engine.clone();
            let cp = cp_base.clone();
            let rid = rid.clone();
            let slug = item.slug.clone();
            async move {
                execute_row(engine, cp, rid, item, user_id).await
            }
            .instrument(tracing::info_span!("execute_row", %slug))
        })
        .buffered(5);

    let full_stream = stream::once(async move { Ok(manifest_event) })
        .chain(stream)
        .chain(stream::iter(end_events));

    Sse::new(full_stream.boxed()).keep_alive(KeepAlive::default())
}

// ─── Scenario Detail (JSON) ────────────────────────────────────────────────

/// GET /api/v1/recommendation/scenario/:slug
/// Direct single-scenario discovery.
pub async fn get_scenario_recommendations(
    Path(slug): Path<String>,
    Query(mut context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Result<axum::Json<StandardResponse<Vec<RecommendationItem>>>, crate::error::AppError> {
    let request_id = extract_request_id(&req);
    let identity = req.extensions().get::<IdentityContext>().cloned();
    
    if let Some(ref id) = identity {
        context_params.merge_identity(id);
    }

    let user_id = context_params.user_id;
    
    let items = execute_and_map(
        engine,
        &slug,
        user_id,
        Some(context_params),
        serde_json::json!({}),
        0,
        20,
        request_id.clone(),
    ).await?;

    Ok(axum::Json(StandardResponse::success(items).with_request_id(request_id)))
}

// ─── Internal Row Execution Helper ─────────────────────────────────────────

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
        let s_map: tokio::sync::RwLockReadGuard<'_, std::collections::HashMap<String, ScenarioDefinition>> = engine.governance.scenarios.scenarios.read().await;
        s_map.get(&slug).map(|s| {
            if s.maturity_rating == "18" {
                cp.maturity_rating.as_deref() == Some("18")
            } else { true }
        }).unwrap_or(true)
    };

    if !is_safe {
        return Ok(Event::default().comment(format!("skip: restricted:{}", slug)));
    }

    let mut result = execute_and_map(
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
        if let Some(ref f_slug) = fallback {
            result = execute_and_map(
                engine.clone(),
                f_slug,
                user_id,
                Some(cp.clone()),
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
                let s_map: tokio::sync::RwLockReadGuard<'_, std::collections::HashMap<String, ScenarioDefinition>> = engine.governance.scenarios.scenarios.read().await;
                s_map.get(&slug).map(|s| s.name.clone()).unwrap_or_else(|| slug.replace('_', " "))
            };
            let row = FeedRow {
                title,
                row_type: comp_item.row_type,
                row_style: comp_item.row_style,
                scenario: slug.clone(),
                scenario_slug: slug,
                items,
            };
            Ok(Event::default().event("row").json_data(&row).unwrap_or_else(|_| Event::default().comment("serial_error")))
        }
        Err(_) => Ok(Event::default().comment(format!("error: failed:{}", slug)))
    }
}
