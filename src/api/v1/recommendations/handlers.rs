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

    // 3. Initial Events Stream (Instant-On)
    let nav_event = Event::default()
        .event("navigation")
        .json_data(serde_json::json!({ "active_pages": active_pages }))
        .unwrap_or_else(|_| Event::default().comment("nav_serialization_error"));

    let manifest_event = Event::default()
        .event("manifest")
        .json_data(serde_json::json!({ 
            "expected_rows": total_scenarios,
            "request_id": rid_orchestrator.clone() 
        }))
        .unwrap_or_else(|_| Event::default().comment("manifest_serialization_error"));

    let initial_stream = stream::iter(vec![Ok(nav_event), Ok(manifest_event)]);

    // 4. Parallel Pipelined Scenario Stream
    let scenario_stream = stream::iter(scenario_slugs)
        .map(move |slug| {
            let engine = engine_clone.clone();
            let cp = cp_base.clone();
            let rid = rid_orchestrator.clone();
            
            async move {
                let result = execute_and_map(
                    engine.clone(),
                    &slug,
                    Some(user_id),
                    Some(cp),
                    serde_json::json!({}),
                    0,
                    20,
                    rid.clone(),
                )
                .instrument(info_span!("scenario_execution", request_id = %rid, scenario = %slug))
                .await;

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
                        
                        Ok(event)
                    }
                    Err(e) => {
                        warn!(request_id = %rid, scenario = %slug, error = ?e, "Scenario failed in Symphony stream, emitting fallback comment");
                        let comment = format!("error: scenario '{}' failed", slug);
                        Ok(Event::default().comment(comment))
                    }
                }
            }
        })
        .buffered(5); // Parallel execution of 5 rows while maintaining order

    // 5. Combine and deliver the Symphony
    let full_stream = initial_stream.chain(scenario_stream);

    Sse::new(full_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
