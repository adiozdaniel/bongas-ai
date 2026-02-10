use anyhow::Result;
use sha2::{Sha256, Digest};
use serde_json::Value as JsonValue;
use tracing::{info, debug};

use crate::cache::redis::RedisClient;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::pipeline::ScoredItem;

pub struct StagingManager {
    redis: RedisClient,
    cache_repo: CacheRepository,
}

impl StagingManager {
    pub async fn new(redis_url: &str, db_pool: sqlx::PgPool) -> Result<Self> {
        Ok(Self {
            redis: RedisClient::new(redis_url).await?,
            cache_repo: CacheRepository::new(db_pool),
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
            info!(cache_key = %cache_key, "L1 cache hit");
            return Ok(Some(items));
        }

        // Try L2 cache (PostgreSQL)
        if let Some(items) = self.get_from_l2(&cache_key).await? {
            info!(cache_key = %cache_key, "L2 cache hit");

            // Promote to L1 cache
            self.save_to_l1(&cache_key, &items, 300).await?;

            return Ok(Some(items));
        }

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

        info!(
            user_id = user_id,
            scenario_slug = ?scenario_slug,
            reason = reason,
            rows_affected = rows_affected,
            "Cache marked as stale"
        );

        Ok(())
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
        // Delete specific key patterns for user
        // In production, use SCAN instead of KEYS for large datasets
        let key = if let Some(slug) = scenario_slug {
            format!("rec:{}:{}:default", slug, user_id)
        } else {
            // Can't easily glob-delete with our RedisClient, so delete known patterns
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
