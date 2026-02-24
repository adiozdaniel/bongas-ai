use std::sync::Arc;
use crate::engine::BongasEngine;
use crate::api::models::{RecommendationItem, ContextParams};
use crate::error::AppError;
use crate::ingestion::types::UserActivity;

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
) -> Result<Vec<RecommendationItem>, AppError> {
    let engine_ref = engine.as_ref();
    
    // Extract context parameters
    let (profile_id, maturity_rating, device_type) = if let Some(cp) = context_params {
        (cp.profile_id, cp.maturity_rating, cp.device_type)
    } else {
        (None, None, None)
    };
    
    // 1. Get Scenario display limit
    let display_limit = {
        let scenarios = engine_ref.scenarios.read().await;
        scenarios.get(scenario_slug)
            .map(|s| s.initial_display_limit as usize)
            .unwrap_or(5)
    };

    // 2. Execute First Window (Instant-On)
    let (items, _stats) = engine_ref
        .execute_scenario_with_stats_contextual(
            scenario_slug, 
            user_id, 
            profile_id.as_ref().map(|s: &String| s.clone()),
            maturity_rating.as_ref().map(|s: &String| s.clone()),
            device_type.as_ref().map(|s: &String| s.clone()),
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
        let activities: Vec<UserActivity> = final_items.iter().map(|item| {
            UserActivity::Impression {
                user_id: uid,
                item_id: item.item_id,
                scenario_slug: Some(scenario_slug.to_string()),
                timestamp: chrono::Utc::now(),
            }
        }).collect();

        // Ingest activities asynchronously
        let engine_clone_for_ingestion = engine_ref.ingestion_manager.clone();
        tokio::spawn(async move {
            let manager = engine_clone_for_ingestion.read().await;
            let api_source = manager.api_source();
            for act in activities {
                let _ = api_source.ingest(act).await;
            }
        });

        // ─── Ecosystem Synergy (Phase 14) ──────────────────────────────────
        let engine_clone_for_synergy = engine.clone();
        let uid = uid;
        let pid = profile_id.clone();
        let slug = scenario_slug.to_string();
        let item_ids: Vec<i32> = final_items.iter().map(|i| i.item_id).collect();
        
        tokio::spawn(async move {
            let manager = engine_clone_for_synergy.ingestion_manager.read().await;
            manager.broadcast_recommendations(uid, pid, slug, item_ids).await;
        });
    }

    // 3. ─── Background Pre-Warming (Phase 12) ───────────────────────────
    if offset == 0 {
        let engine_clone_for_warming = engine.clone();
        let slug = scenario_slug.to_string();
        let ctx = context_data.clone();
        
        tokio::spawn(async move {
            // We force a refresh of the cache by executing the scenario with a larger internal limit (None)
            let _ = engine_clone_for_warming.execute_scenario_with_stats(&slug, user_id, ctx, None).await;
        });
    }

    Ok(final_items)
}
