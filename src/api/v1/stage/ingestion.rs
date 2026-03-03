//! Ingestion sub-module for the Symphony Stage.
//! Handles frictionless behavior tracking and context enrichment.

use axum::{
    extract::Extension,
    http::HeaderMap,
    Json,
};
use std::sync::Arc;
use tokio::spawn;

use crate::engine::BongasEngine;
use crate::api::models::StandardResponse;
use crate::api::models::recommendation::IngestEvent;
use crate::api::middleware::service::extract_request_id_from_headers;
use crate::api::middleware::identity::IdentityContext;
use crate::ingestion::types::UserActivity;

/// POST /api/v1/recommendation/ingest
/// Frictionless entry point for client-side behavior tracking.
pub async fn ingest_activities(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(identity): Extension<IdentityContext>,
    headers: HeaderMap,
    Json(events): Json<Vec<IngestEvent>>,
) -> Json<StandardResponse<()>> {
    let request_id = extract_request_id_from_headers(&headers);
    let vid = Some(identity.visitor_id.clone());
    let dhash = Some(identity.device_hash.clone());
    let dtype = Some(identity.device_type.clone());

    for ev in events {
        let activity = match ev.event.as_str() {
            "click" => UserActivity::Click {
                user_id: ev.user_id.unwrap_or(0),
                item_id: ev.item_id,
                visitor_id: vid.clone(),
                device_hash: dhash.clone(),
                device_type: dtype.clone(),
                scenario_slug: ev.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
            "playback" => UserActivity::Playback {
                user_id: ev.user_id.unwrap_or(0),
                item_id: ev.item_id,
                session_id: "api_ingest".to_string(),
                visitor_id: vid.clone(),
                device_hash: dhash.clone(),
                device_type: dtype.clone(),
                watch_duration_seconds: 0,
                total_duration_seconds: 0,
                watch_percentage: ev.watch_percentage.unwrap_or(0.0),
                completed: ev.watch_percentage.unwrap_or(0.0) > 0.9,
                scenario_slug: ev.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
            _ => continue,
        };

        let engine_inner = engine.clone();
        spawn(async move {
            let manager = engine_inner.ingestion.read().await;
            manager.api_source().ingest(activity).await;
        });
    }

    Json(StandardResponse::success(()).with_request_id(request_id))
}
