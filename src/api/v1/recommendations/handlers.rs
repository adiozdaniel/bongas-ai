use axum::{
    extract::{Path, Extension, Query},
    response::sse::{Event, Sse, KeepAlive},
    body::Body,
    http::Request,
};
use std::time::Duration;
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{warn, info_span, Instrument};

use crate::engine::BongasEngine;
use crate::api::models::ContextParams;
use crate::api::models::recommendation::FeedRow;
use crate::api::middleware::service::extract_request_id;
use crate::api::middleware::identity::IdentityContext;
use crate::pages::types::PageCompositionItem;
use super::service::execute_and_map;

/// The Master Orchestrator for the Bongas-AI Symphony.
/// Delivers high-performance, parallel-pipelined Server-Driven UI (SDUI) content.
pub async fn get_page_recommendations(
    Path((page_slug, user_id)): Path<(String, i32)>,
    Query(mut context_params): Query<ContextParams>,
    Extension(engine): Extension<Arc<BongasEngine>>,
    req: Request<Body>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = extract_request_id(&req);
    let identity = req.extensions().get::<IdentityContext>().cloned();
    
    // 1. Identity Stitching (Zero-Touch)
    if let Some(ref id) = identity {
        context_params.merge_identity(id);
    }

    let engine_clone = engine.clone();
    let cp_base = context_params.clone();
    let rid_orchestrator = request_id.clone();

    // 2. Resolve Dynamic Page Layout (Targeting Resolver + The Brain)
    let identity_key = cp_base.visitor_id.as_deref().unwrap_or(&user_id.to_string()).to_string();
    let active_pages = engine.pages.list_active_pages().await.unwrap_or_default();
    let layout_res = engine.pages.get_layout_contextual(
        &page_slug, 
        cp_base.device_type.as_deref(), 
        cp_base.maturity_rating.as_deref(),
        Some(&identity_key)
    ).await;
    
    let composition = match layout_res {
        Ok(Some(layout)) => layout.composition,
        _ => {
            if page_slug == "home" {
                warn!(request_id = %request_id, "Home layout not found for context, using default fallback symphony");
                vec![
                    PageCompositionItem { slug: "trending_now".into(), row_type: "hero_carousel".into(), row_style: Some("promotional".into()) },
                    PageCompositionItem { slug: "personalized_picks".into(), row_type: "horizontal_list".into(), row_style: Some("standard".into()) },
                    PageCompositionItem { slug: "home_feed".into(), row_type: "horizontal_list".into(), row_style: Some("standard".into()) },
                ]
            } else {
                warn!(request_id = %request_id, page = %page_slug, "Page layout not found, stream will be empty");
                vec![]
            }
        }
    };

    let total_scenarios = composition.len();

    // 3. Initial Events Stream (Instant-On)
    let nav_event = Event::default()
        .event("navigation")
        .json_data(serde_json::json!({ "active_pages": active_pages }))
        .unwrap_or_else(|_| Event::default().comment("nav_serialization_error"));

    let manifest_event = Event::default()
        .event("manifest")
        .json_data(serde_json::json!({ 
            "expected_rows": total_scenarios,
            "request_id": rid_orchestrator.clone(),
            "page": page_slug
        }))
        .unwrap_or_else(|_| Event::default().comment("manifest_serialization_error"));

    let initial_stream = stream::iter(vec![Ok(nav_event), Ok(manifest_event)]);

    // 4. Parallel Pipelined SDUI Stream
    let scenario_stream = stream::iter(composition)
        .map(move |comp_item| {
            let engine = engine_clone.clone();
            let cp = cp_base.clone();
            let rid = rid_orchestrator.clone();
            let scenario_slug = comp_item.slug.clone();
            let row_type = comp_item.row_type.clone();
            let row_style = comp_item.row_style.clone();
            
            async move {
                // 4.1 Early Safety Check (Maturity Ceiling)
                let is_safe = {
                    let scenarios = engine.scenarios.scenarios.read().await;
                    scenarios.get(&scenario_slug).map(|s| {
                        // Logic: If scenario is '18', user MUST be '18'. 
                        // If scenario is 'all', everyone can see it.
                        if s.maturity_rating == "18" {
                            cp.maturity_rating.as_deref() == Some("18")
                        } else {
                            true
                        }
                    }).unwrap_or(true) // If scenario not found, let it fail in execution
                };

                if !is_safe {
                    warn!(request_id = %rid, scenario = %scenario_slug, "Scenario blocked by maturity safety ceiling");
                    return Ok(Event::default().comment(format!("safety: scenario '{}' restricted", scenario_slug)));
                }

                let result = execute_and_map(
                    engine.clone(),
                    &scenario_slug,
                    Some(user_id),
                    Some(cp),
                    serde_json::json!({}),
                    0,
                    20,
                    rid.clone(),
                )
                .instrument(info_span!("scenario_execution", request_id = %rid, scenario = %scenario_slug))
                .await;

                match result {
                    Ok(items) => {
                        let title = {
                            let scenarios = engine.scenarios.scenarios.read().await;
                            scenarios.get(&scenario_slug)
                                .map(|s| s.name.clone())
                                .unwrap_or_else(|| scenario_slug.replace('_', " "))
                        };

                        let row = FeedRow {
                            title,
                            row_type,
                            row_style,
                            scenario: scenario_slug,
                            items,
                        };

                        let event = Event::default()
                            .event("row")
                            .json_data(&row)
                            .unwrap_or_else(|_| Event::default().comment("serialization_error"));
                        
                        Ok(event)
                    }
                    Err(e) => {
                        warn!(request_id = %rid, scenario = %scenario_slug, error = ?e, "Scenario failed in Symphony stream, emitting fallback comment");
                        let comment = format!("error: scenario '{}' failed", scenario_slug);
                        Ok(Event::default().comment(comment))
                    }
                }
            }
        })
        .buffered(5);

    let full_stream = initial_stream.chain(scenario_stream);

    Sse::new(full_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
