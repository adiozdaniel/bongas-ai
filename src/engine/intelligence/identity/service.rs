//! Identity intelligence service for reactive stitching and profile management.

use std::sync::Arc;
use tracing::{info, debug, error};
use crate::engine::coordination::service::BongasEngine;
use crate::error::AppResult;

/// Orchestrates reactive identity stitching and profile-aware security.
pub struct IdentityStitcher {
    // Shared state if needed
}

impl IdentityStitcher {
    pub fn new() -> Self {
        Self {}
    }

    /// Reactively detect and trigger identity stitching if needed.
    /// Uses Redis to ensure we only stitch once per profile/device pair.
    pub async fn reactive_stitch(
        &self,
        engine: Arc<BongasEngine>,
        visitor_id: &str,
        user_id: i32,
        profile_id: &str,
    ) -> AppResult<()> {
        let cache_key = format!("stitched:{}:{}", visitor_id, profile_id);
        
        // 1. Check if already stitched in Redis (high-speed guard)
        if let Ok(Some(_)) = engine.cache.get::<String>(&cache_key, "identity_stitch", Some(user_id), Some(profile_id)).await {
            return Ok(());
        }

        info!(
            visitor_id = %visitor_id, 
            user_id = user_id, 
            profile_id = %profile_id, 
            "Identity transition detected, triggering background stitch"
        );

        let engine_clone = engine.clone();
        let vid = visitor_id.to_string();
        let pid = profile_id.to_string();

        // 2. Spawn background stitch task
        tokio::spawn(async move {
            // A. Update Interaction History
            match engine_clone.execution.interaction_repo.stitch_identity(&vid, user_id, &pid).await {
                Ok(rows) => debug!(rows, "Interactions stitched"),
                Err(e) => error!(error = %e, "Failed to stitch interactions"),
            }

            // B. Merge ML Features
            let _ = engine_clone.execution.feature_repo.merge_visitor_features(&vid, user_id, &pid).await;

            // C. Mark as Stitched in Redis (24h TTL)
            let _ = engine_clone.cache.set_with_ttl(
                &cache_key, 
                &"1".to_string(), 
                std::time::Duration::from_secs(86400),
                "identity_stitch",
                Some(user_id),
                Some(&pid)
            ).await;
            
            // D. Invalidate Caches
            let pattern = format!("ghost:user_*:page_*:offset_*");
            let _ = engine_clone.cache.delete_pattern(&pattern).await;
            
            info!(visitor_id = %vid, profile_id = %pid, "Identity stitch completed");
        });

        Ok(())
    }
}
