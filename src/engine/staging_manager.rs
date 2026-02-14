//! Staging manager for recommendation caching with Netflix-grade patterns.
//!
//! Architecture:
//! - L1: In-memory LRU cache (via CacheManager)
//! - L2: Redis cache with circuit breaker (via CacheManager)
//! - L3: PostgreSQL for persistence (via CacheRepository)

use anyhow::Result;
use sha2::{Sha256, Digest};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, debug};

use crate::cache::{CacheManager, CacheConfig, CacheMetricsSnapshot};
use crate::db::repositories::cache_repository::CacheRepository;
use crate::pipeline::ScoredItem;

pub struct StagingManager {
    cache_manager: Arc<CacheManager>,
    cache_repo: CacheRepository,
}

impl StagingManager {
    pub async fn new(redis_url: &str, db_pool: sqlx::PgPool, config: CacheConfig) -> Result<Self> {
        let cache_manager = Arc::new(CacheManager::new(redis_url, config).await?);

        Ok(Self {
            cache_manager,
            cache_repo: CacheRepository::new(db_pool),
        })
    }

    /// Get recommendations from cache tiers (L1 -> L2 -> L3)
    pub async fn get_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: &str,
    ) -> Result<Option<Vec<ScoredItem>>> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, context_hash);
        let start_time = Instant::now();

        // Try L1 (LRU) and L2 (Redis) via CacheManager
        if let Some(items) = self.cache_manager.get::<Vec<ScoredItem>>(&cache_key).await? {
            let _duration = start_time.elapsed();
            info!(cache_key = %cache_key, "Cache hit (L1/L2)");
            return Ok(Some(items));
        }

        // Try L3 (PostgreSQL)
        if let Some(items) = self.get_from_l3(&cache_key).await? {
            let _duration = start_time.elapsed();
            info!(cache_key = %cache_key, "L3 cache hit (PostgreSQL)");

            // Promote to L1/L2
            let _ = self.cache_manager.set(&cache_key, &items).await;

            return Ok(Some(items));
        }

        let _duration = start_time.elapsed();
        debug!(cache_key = %cache_key, "Cache miss");
        Ok(None)
    }

    /// Save recommendations to all cache tiers
    pub async fn save_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: &str,
        items: &[ScoredItem],
        ttl_seconds: i32,
    ) -> Result<()> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, context_hash);

        // Save to L1/L2 via CacheManager
        self.cache_manager.set(&cache_key, &items.to_vec()).await?;

        // Save to L3 (PostgreSQL) with longer TTL (12x)
        let l3_ttl = ttl_seconds * 12;
        let items_json = serde_json::to_value(items)?;
        self.cache_repo.set(
            &cache_key,
            scenario_slug,
            user_id,
            Some(context_hash),
            items_json,
            l3_ttl,
        ).await?;

        info!(
            cache_key = %cache_key,
            l3_ttl = l3_ttl,
            "Saved to all cache tiers"
        );

        Ok(())
    }

    /// Mark cache entries as stale (called by Staleness Engine)
    pub async fn mark_stale(
        &self,
        user_id: i32,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> Result<()> {
        // Invalidate L1/L2 via pattern (best effort)
        let key_pattern = if let Some(slug) = scenario_slug {
            format!("rec:{}:{}:*", slug, user_id)
        } else {
            format!("rec:*:{}:*", user_id)
        };
        let _ = self.cache_manager.delete(&key_pattern).await;

        // Mark L3 cache as stale
        let rows_affected = self.cache_repo.mark_stale(user_id, scenario_slug, reason).await?;

        info!(
            user_id = user_id,
            scenario_slug = ?scenario_slug,
            reason = reason,
            rows_affected = rows_affected,
            "Cache marked as stale"
        );

        Ok(())
    }

    /// Invalidate cache for a specific scenario + user
    pub async fn invalidate(
        &self,
        scenario_slug: &str,
        user_id: i32,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Invalidate L1/L2 - specific key patterns
        let key = format!("rec:{}:{}:*", scenario_slug, user_id);
        let _ = self.cache_manager.delete(&key).await;

        let default_key = format!("rec:{}:{}:default", scenario_slug, user_id);
        let _ = self.cache_manager.delete(&default_key).await;

        // Invalidate L3 (PostgreSQL)
        let rows_affected = self.cache_repo.mark_stale(user_id, Some(scenario_slug), "invalidate").await?;

        let _duration = start_time.elapsed();

        info!(
            scenario_slug = scenario_slug,
            user_id = user_id,
            rows_affected = rows_affected,
            "Invalidated all cache tiers"
        );

        Ok(())
    }

    /// Cleanup expired L3 cache entries
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let deleted = self.cache_repo.cleanup_expired().await?;
        info!(deleted = deleted, "Cleaned up expired L3 cache entries");
        Ok(deleted)
    }

    /// Get cache hit rate from CacheManager metrics
    pub fn get_hit_rate(&self) -> f64 {
        self.cache_manager.metrics().overall_hit_rate()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> StagingStats {
        let metrics = self.cache_manager.metrics();
        StagingStats {
            l1_hits: metrics.l1_hits,
            l1_misses: metrics.l1_misses,
            l2_hits: metrics.l2_hits,
            l2_misses: metrics.l2_misses,
            invalidations: 0, // TODO: Track via CacheMetrics
            hit_rate: metrics.overall_hit_rate(),
        }
    }

    /// Get the underlying CacheManager for direct access
    pub fn cache_manager(&self) -> Arc<CacheManager> {
        Arc::clone(&self.cache_manager)
    }

    /// Get from PostgreSQL L3 cache
    async fn get_from_l3(&self, cache_key: &str) -> Result<Option<Vec<ScoredItem>>> {
        if let Some(entry) = self.cache_repo.get(cache_key).await? {
            let items: Vec<ScoredItem> = serde_json::from_value(entry.recommendations)?;
            return Ok(Some(items));
        }
        Ok(None)
    }

    /// Build cache key
    fn build_cache_key(scenario_slug: &str, user_id: Option<i32>, context_hash: &str) -> String {
        let user_part = user_id.map(|id| id.to_string()).unwrap_or_else(|| "anon".to_string());
        format!("rec:{}:{}:{}", scenario_slug, user_part, context_hash)
    }

    /// Hash context parameters
    pub fn hash_context(params: &JsonValue) -> String {
        let mut hasher = Sha256::new();
        hasher.update(params.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StagingStats {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub invalidations: u64,
    pub hit_rate: f64,
}
