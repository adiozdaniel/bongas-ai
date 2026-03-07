//! Ingestion sub-module for the Symphony Stage.
//! Handles frictionless behavior tracking and real-time event streaming.

use axum::{
    extract::Extension,
    Json,
    http::HeaderMap,
};
use std::sync::Arc;
use tokio::spawn;

use crate::engine::coordination::service::BongasEngine;
use crate::api::{StandardResponse, IngestEvent};
use crate::api::extract_request_id_from_headers;
use crate::api::IdentityContext;
use crate::ingestion::types::UserActivity;

/// POST /api/v1/recommendation/ingest
/// Frictionless feedback loop. Ingests user interactions (clicks, playbacks, reactions).
pub async fn ingest_activity(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(identity): Extension<IdentityContext>,
    headers: HeaderMap,
    Json(payload): Json<IngestEvent>,
) -> Json<StandardResponse<()>> {
    let request_id = extract_request_id_from_headers(&headers);
    
    // Convert to internal activity enum
    let activity = match payload.event.as_str() {
        "click" => UserActivity::Click {
            user_id: payload.user_id.unwrap_or(0),
            profile_id: identity.profile_id.clone(),
            item_id: payload.item_id,
            visitor_id: Some(identity.visitor_id.clone()),
            device_hash: Some(identity.device_hash.clone()),
            device_type: Some(identity.device_type.clone()),
            scenario_slug: payload.scenario.clone(),
            timestamp: chrono::Utc::now(),
        },
        "playback" => UserActivity::Playback {
            user_id: payload.user_id.unwrap_or(0),
            profile_id: identity.profile_id.clone(),
            item_id: payload.item_id,
            session_id: uuid::Uuid::new_v4().to_string(),
            visitor_id: Some(identity.visitor_id.clone()),
            device_hash: Some(identity.device_hash.clone()),
            device_type: Some(identity.device_type.clone()),
            watch_duration_seconds: payload.watch_percentage.map(|p| (p * 60.0) as i32).unwrap_or(0),
            total_duration_seconds: 60,
            watch_percentage: payload.watch_percentage.unwrap_or(0.0),
            completed: payload.watch_percentage.map(|p| p > 0.9).unwrap_or(false),
            scenario_slug: payload.scenario.clone(),
            timestamp: chrono::Utc::now(),
        },
        "reaction" => UserActivity::Reaction {
            user_id: payload.user_id.unwrap_or(0),
            profile_id: identity.profile_id.clone(),
            item_id: payload.item_id,
            visitor_id: Some(identity.visitor_id.clone()),
            device_hash: Some(identity.device_hash.clone()),
            device_type: Some(identity.device_type.clone()),
            reaction_type: payload.reaction_type.unwrap_or_else(|| "like".to_string()),
            scenario_slug: payload.scenario.clone(),
            timestamp: chrono::Utc::now(),
        },
        _ => UserActivity::Impression {
            user_id: payload.user_id.unwrap_or(0),
            profile_id: identity.profile_id.clone(),
            item_id: payload.item_id,
            visitor_id: Some(identity.visitor_id.clone()),
            device_hash: Some(identity.device_hash.clone()),
            device_type: Some(identity.device_type.clone()),
            scenario_slug: payload.scenario.clone(),
            timestamp: chrono::Utc::now(),
        },
    };

    let engine_inner = engine.clone();
    spawn(async move {
        let manager = engine_inner.ingestion.read().await;
        manager.api_source().ingest(activity).await;
    });

    Json(StandardResponse::success(()).with_request_id(request_id))
}

/// POST /api/v1/recommendation/ingest/batch
pub async fn ingest_activities(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Extension(identity): Extension<IdentityContext>,
    headers: HeaderMap,
    Json(payload): Json<Vec<IngestEvent>>,
) -> Json<StandardResponse<()>> {
    let request_id = extract_request_id_from_headers(&headers);

    for event in payload {
        let activity = match event.event.as_str() {
            "click" => UserActivity::Click {
                user_id: event.user_id.unwrap_or(0),
                profile_id: identity.profile_id.clone(),
                item_id: event.item_id,
                visitor_id: Some(identity.visitor_id.clone()),
                device_hash: Some(identity.device_hash.clone()),
                device_type: Some(identity.device_type.clone()),
                scenario_slug: event.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
            "playback" => UserActivity::Playback {
                user_id: event.user_id.unwrap_or(0),
                profile_id: identity.profile_id.clone(),
                item_id: event.item_id,
                session_id: uuid::Uuid::new_v4().to_string(),
                visitor_id: Some(identity.visitor_id.clone()),
                device_hash: Some(identity.device_hash.clone()),
                device_type: Some(identity.device_type.clone()),
                watch_duration_seconds: event.watch_percentage.map(|p| (p * 60.0) as i32).unwrap_or(0),
                total_duration_seconds: 60,
                watch_percentage: event.watch_percentage.unwrap_or(0.0),
                completed: event.watch_percentage.map(|p| p > 0.9).unwrap_or(false),
                scenario_slug: event.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
            "reaction" => UserActivity::Reaction {
                user_id: event.user_id.unwrap_or(0),
                profile_id: identity.profile_id.clone(),
                item_id: event.item_id,
                visitor_id: Some(identity.visitor_id.clone()),
                device_hash: Some(identity.device_hash.clone()),
                device_type: Some(identity.device_type.clone()),
                reaction_type: event.reaction_type.unwrap_or_else(|| "like".to_string()),
                scenario_slug: event.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
            _ => UserActivity::Impression {
                user_id: event.user_id.unwrap_or(0),
                profile_id: identity.profile_id.clone(),
                item_id: event.item_id,
                visitor_id: Some(identity.visitor_id.clone()),
                device_hash: Some(identity.device_hash.clone()),
                device_type: Some(identity.device_type.clone()),
                scenario_slug: event.scenario.clone(),
                timestamp: chrono::Utc::now(),
            },
        };

        let engine_inner = engine.clone();
        spawn(async move {
            let manager = engine_inner.ingestion.read().await;
            manager.api_source().ingest(activity).await;
        });
    }

    Json(StandardResponse::success(()).with_request_id(request_id))
}
