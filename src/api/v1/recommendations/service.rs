//! Service layer for recommendation orchestration and mapping.

use std::sync::Arc;
use crate::engine::BongasEngine;
use crate::api::models::{RecommendationItem, ContextParams};
use crate::error::AppError;
use crate::ingestion::types::UserActivity;

use tracing::{Instrument, info_span};

/// Execute a scenario and map engine items to API RecommendationItems.
/// This service orchestrates engine execution, mapping, impression tracking, and pre-warming.
pub async fn execute_and_map(
    engine: Arc<BongasEngine>,
    scenario_slug: &str,
    user_id: Option<i32>,
    context_params: Option<ContextParams>,
    context_data: serde_json::Value,
    offset: usize,
    _limit: usize,
    request_id: String,
) -> Result<Vec<RecommendationItem>, AppError> {
    let engine_ref = engine.as_ref();
    
    // Extract context parameters
    let (profile_id, maturity_rating, device_type, visitor_id, device_hash) = if let Some(ref cp) = context_params {
        (
            cp.profile_id.clone(), 
            cp.maturity_rating.clone(), 
            cp.device_type.clone(),
            cp.visitor_id.clone(),
            cp.device_hash.clone()
        )
    } else {
        (None, None, None, None, None)
    };
    
    // 1. Get Scenario display limit
    let display_limit = {
        let scenarios = engine_ref.scenarios.scenarios.read().await;
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
        let vid = visitor_id.clone();
        let dhash = device_hash.clone();
        let slug_clone = scenario_slug.to_string();
        
        let activities: Vec<UserActivity> = final_items.iter().map(|item| {
            UserActivity::Impression {
                user_id: uid,
                item_id: item.item_id,
                visitor_id: vid.clone(),
                device_hash: dhash.clone(),
                scenario_slug: Some(slug_clone.clone()),
                timestamp: chrono::Utc::now(),
            }
        }).collect();

        // Ingest activities asynchronously - Tied to Request ID
        let engine_clone_for_ingestion = engine_ref.ingestion_manager.clone();
        let rid_ingest = request_id.clone();
        tokio::spawn(async move {
            let manager = engine_clone_for_ingestion.read().await;
            let api_source = manager.api_source();
            for act in activities {
                let _ = api_source.ingest(act).await;
            }
        }.instrument(info_span!("async_impression_ingestion", request_id = %rid_ingest)));

        // ─── Ecosystem Synergy (Phase 14) ──────────────────────────────────
        let engine_clone_for_synergy = engine.clone();
        let slug = scenario_slug.to_string();
        let item_ids: Vec<i32> = final_items.iter().map(|i| i.item_id).collect();
        let rid_synergy = request_id.clone();
        let pid_clone = profile_id.clone();
        
        tokio::spawn(async move {
            let manager = engine_clone_for_synergy.ingestion_manager.read().await;
            manager.broadcast_recommendations(uid, pid_clone, slug, item_ids).await;
        }.instrument(info_span!("async_ecosystem_synergy", request_id = %rid_synergy)));
    }

    // 3. ─── Background Pre-Warming (Phase 12) ───────────────────────────
    if offset == 0 {
        let engine_clone_for_warming = engine.clone();
        let slug = scenario_slug.to_string();
        let ctx = context_data.clone();
        let rid_warming = request_id.clone();
        let slug_for_span = slug.clone();
        
        tokio::spawn(async move {
            let _ = engine_clone_for_warming.execute_scenario_with_stats(&slug, user_id, ctx, None).await;
        }.instrument(info_span!("async_pre_warming", request_id = %rid_warming, scenario = %slug_for_span)));
    }

    Ok(final_items)
}
