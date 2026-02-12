use anyhow::Result;
use sha2::{Sha256, Digest};
use serde_json::Value as JsonValue;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{info, debug};

use crate::cache::redis::RedisClient;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::pipeline::ScoredItem;

pub struct StagingManager {
    redis: RedisClient,
    cache_repo: CacheRepository,
    // Metrics counters
    l1_hits: AtomicU64,
    l1_misses: AtomicU64,
    l2_hits: AtomicU64,
    l2_misses: AtomicU64,
    invalidations: AtomicU64,
}

impl StagingManager {
    pub async fn new(redis_url: &str, db_pool: sqlx::PgPool) -> Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url).await?,
            cache_repo: CacheRepository::new(db_pool),
            l1_hits: AtomicU64::new(0),
            l1_misses: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l2_misses: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
        })
    }

    /// Get recommendations from L1 (Redis) or L2 (PostgreSQL) cache
    pub async fn get_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: &str,
    ) -> Result<Option<Vec<ScoredItem>>> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, context_hash);

        // Try L1 cache (Redis) first
        if let Some(items) = self.get_from_l1(&cache_key).await? {
            self.l1_hits.fetch_add(1, Ordering::Relaxed);
            info!(cache_key = %cache_key, "L1 cache hit");
            return Ok(Some(items));
        }
        self.l1_misses.fetch_add(1, Ordering::Relaxed);

        // Try L2 cache (PostgreSQL)
        if let Some(items) = self.get_from_l2(&cache_key).await? {
            self.l2_hits.fetch_add(1, Ordering::Relaxed);
            info!(cache_key = %cache_key, "L2 cache hit");

            // Promote to L1 cache
            self.save_to_l1(&cache_key, &items, 300).await?;

            return Ok(Some(items));
        }
        self.l2_misses.fetch_add(1, Ordering::Relaxed);

        debug!(cache_key = %cache_key, "Cache miss");
        Ok(None)
    }

    /// Save recommendations to both L1 and L2 caches
    pub async fn save_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: &str,
        items: &[ScoredItem],
        ttl_seconds: i32,
    ) -> Result<()> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, context_hash);

        // Save to L1 (Redis) with short TTL
        self.save_to_l1(&cache_key, items, ttl_seconds).await?;

        // Save to L2 (PostgreSQL) with longer TTL (12x)
        let l2_ttl = ttl_seconds * 12;
        let items_json = serde_json::to_value(items)?;
        self.cache_repo.set(
            &cache_key,
            scenario_slug,
            user_id,
            Some(context_hash),
            items_json,
            l2_ttl,
        ).await?;

        info!(
            cache_key = %cache_key,
            l1_ttl = ttl_seconds,
            l2_ttl = l2_ttl,
            "Saved to L1 and L2 caches"
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
        // Invalidate L1 cache (Redis)
        self.invalidate_l1_for_user(user_id, scenario_slug).await?;

        // Mark L2 cache as stale
        let rows_affected = self.cache_repo.mark_stale(user_id, scenario_slug, reason).await?;
        self.invalidations.fetch_add(1, Ordering::Relaxed);

        info!(
            user_id = user_id,
            scenario_slug = ?scenario_slug,
            reason = reason,
            rows_affected = rows_affected,
            "Cache marked as stale"
        );

        Ok(())
    }

    /// Invalidate both L1 and L2 for a specific scenario + user
    pub async fn invalidate(
        &self,
        scenario_slug: &str,
        user_id: i32,
    ) -> Result<()> {
        // Invalidate L1 (Redis) - specific key pattern
        let key = format!("rec:{}:{}:*", scenario_slug, user_id);
        let _ = self.redis.del(&key).await;

        // Also try the default context hash key
        let default_key = format!("rec:{}:{}:default", scenario_slug, user_id);
        let _ = self.redis.del(&default_key).await;

        // Invalidate L2 (PostgreSQL)
        let rows_affected = self.cache_repo.mark_stale(user_id, Some(scenario_slug), "invalidate").await?;
        self.invalidations.fetch_add(1, Ordering::Relaxed);

        info!(
            scenario_slug = scenario_slug,
            user_id = user_id,
            rows_affected = rows_affected,
            "Invalidated L1 and L2 caches"
        );

        Ok(())
    }

    /// Invalidate all caches for a user (all scenarios)
    pub async fn invalidate_profile(&self, user_id: i32) -> Result<()> {
        // Invalidate L1 (Redis) - pattern for all scenarios
        let key = format!("rec:*:{}:*", user_id);
        let _ = self.redis.del(&key).await;

        // Invalidate L2 (PostgreSQL) - all scenarios for user
        let rows_affected = self.cache_repo.mark_stale(user_id, None, "profile_invalidate").await?;
        self.invalidations.fetch_add(1, Ordering::Relaxed);

        info!(
            user_id = user_id,
            rows_affected = rows_affected,
            "Invalidated all caches for profile"
        );

        Ok(())
    }

    /// Cleanup expired L2 cache entries
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let deleted = self.cache_repo.cleanup_expired().await?;
        info!(deleted = deleted, "Cleaned up expired L2 cache entries");
        Ok(deleted)
    }

    /// Get cache hit rate
    pub fn get_hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits.load(Ordering::Relaxed)
            + self.l2_hits.load(Ordering::Relaxed);
        let total_misses = self.l1_misses.load(Ordering::Relaxed)
            + self.l2_misses.load(Ordering::Relaxed);

        if total_hits + total_misses == 0 {
            return 0.0;
        }

        total_hits as f64 / (total_hits + total_misses) as f64
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> StagingStats {
        StagingStats {
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.l1_misses.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.l2_misses.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            hit_rate: self.get_hit_rate(),
        }
    }

    /// Get from Redis L1 cache
    async fn get_from_l1(&self, cache_key: &str) -> Result<Option<Vec<ScoredItem>>> {
        if let Some(json_str) = self.redis.get(cache_key).await? {
            let items: Vec<ScoredItem> = serde_json::from_str(&json_str)?;
            return Ok(Some(items));
        }
        Ok(None)
    }

    /// Save to Redis L1 cache
    async fn save_to_l1(&self, cache_key: &str, items: &[ScoredItem], ttl_seconds: i32) -> Result<()> {
        let json_str = serde_json::to_string(items)?;
        self.redis.set_ex(cache_key, &json_str, ttl_seconds as u64).await?;
        Ok(())
    }

    /// Get from PostgreSQL L2 cache
    async fn get_from_l2(&self, cache_key: &str) -> Result<Option<Vec<ScoredItem>>> {
        if let Some(entry) = self.cache_repo.get(cache_key).await? {
            let items: Vec<ScoredItem> = serde_json::from_value(entry.recommendations)?;
            return Ok(Some(items));
        }
        Ok(None)
    }

    /// Invalidate L1 cache for user
    async fn invalidate_l1_for_user(&self, user_id: i32, scenario_slug: Option<&str>) -> Result<()> {
        let key = if let Some(slug) = scenario_slug {
            format!("rec:{}:{}:default", slug, user_id)
        } else {
            format!("rec:*:{}:*", user_id)
        };

        // Best-effort deletion
        let _ = self.redis.del(&key).await;
        Ok(())
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
