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
use tracing::{info, warn, info_span, Instrument};

use crate::engine::BongasEngine;
use crate::api::models::{ContextParams, ContextParams as cp_alias}; // Alias for clarity in closures
use crate::api::models::recommendation::FeedRow;
use crate::api::middleware::service::extract_request_id;
use crate::api::middleware::identity::IdentityContext;
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

    // 2. Resolve Page Layout (The Blueprint)
    let active_pages = engine.pages.list_active_pages().await.unwrap_or_default();
    let layout_res = engine.pages.get_layout(&page_slug).await;
    
    let scenario_slugs = match layout_res {
        Ok(Some(layout)) => layout.scenario_slugs,
        _ => {
            if page_slug == "home" {
                warn!(request_id = %request_id, "Home layout not found, using default fallback symphony");
                vec!["trending_now".to_string(), "personalized_picks".to_string(), "home_feed".to_string()]
            } else {
                warn!(request_id = %request_id, page = %page_slug, "Page layout not found, stream will be empty");
                vec![]
            }
        }
    };

    let total_scenarios = scenario_slugs.len();

    // 3. The Symphony Stream Machine
    let stream = stream::unfold(
        (true, true, active_pages, scenario_slugs), // State: (send_nav, send_manifest, navigation, remaining_slugs)
        move |(send_nav, send_manifest, nav, mut slugs)| {
            let engine = engine_clone.clone();
            let cp_inner = cp_base.clone();
            let rid_inner = rid_orchestrator.clone();
            
            async move {
                // Event 1: Navigation (Instant-On)
                if send_nav {
                    let event = Event::default()
                        .event("navigation")
                        .json_data(serde_json::json!({ "active_pages": nav }))
                        .unwrap_or_else(|_| Event::default().comment("nav_serialization_error"));
                    return Some((Ok(event), (false, true, vec![], slugs)));
                }

                // Event 2: Manifest (Skeleton UI Hints)
                if send_manifest {
                    let event = Event::default()
                        .event("manifest")
                        .json_data(serde_json::json!({ 
                            "expected_rows": total_scenarios,
                            "request_id": rid_inner 
                        }))
                        .unwrap_or_else(|_| Event::default().comment("manifest_serialization_error"));
                    return Some((Ok(event), (false, false, vec![], slugs)));
                }

                if slugs.is_empty() {
                    return None;
                }

                // 4. Execution Fan-Out (Parallel Processing for current row)
                // Note: In Phase 2.1, we will buffer multiple scenarios. For now, we execute 
                // the current row with full context propagation.
                let slug = slugs.remove(0);
                
                let result = execute_and_map(
                    engine.clone(),
                    &slug,
                    Some(user_id),
                    Some(cp_inner.clone()),
                    serde_json::json!({}),
                    0,
                    20,
                    rid_inner.clone(),
                ).instrument(info_span!("scenario_execution", request_id = %rid_inner, scenario = %slug)).await;

                match result {
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
                        
                        Some((Ok(event), (false, false, vec![], slugs)))
                    }
                    Err(e) => {
                        warn!(request_id = %rid_inner, scenario = %slug, error = ?e, "Scenario failed in Symphony stream, emitting fallback comment");
                        let comment = format!("error: scenario '{}' failed", slug);
                        Some((Ok(Event::default().comment(comment)), (false, false, vec![], slugs)))
                    }
                }
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
