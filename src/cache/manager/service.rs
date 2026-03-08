//! Netflix-grade cache manager with L1 (LRU) + L2 (Redis) + L3 (Postgres) composite pattern.

use crate::cache::CacheConfig;
use crate::cache::{CacheMetrics, CacheMetricsSnapshot};
use crate::cache::{LruCache, RedisCache, CacheLayer};
use crate::cache::CacheStrategy;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::RedisConfig;
use crate::db::CacheRepository;

/// Netflix-grade cache manager with multi-tier caching.
///
/// Architecture:
/// - L1: In-memory LRU (fast, local)
/// - L2: Redis (distributed, high-velocity)
/// - L3: Postgres (persistent, system-of-record)
pub struct CacheManager {
    l1: Option<CacheLayer>,
    l2: Option<CacheLayer>,
    l3: Option<Arc<CacheRepository>>,
    config: CacheConfig,
    metrics: Arc<CacheMetrics>,
}

impl CacheManager {
    /// Create a new cache manager with three tiers.
    pub async fn new(
        redis_config: RedisConfig,
        config: CacheConfig,
        l3_repo: Option<Arc<CacheRepository>>,
    ) -> Result<Self> {
        let metrics = Arc::new(CacheMetrics::new());

        let l1 = if config.l1_enabled {
            Some(CacheLayer::Lru(LruCache::new(
                config.l1_max_entries,
                Arc::clone(&metrics),
            )))
        } else {
            None
        };

        let l2 = if config.l2_enabled {
            match RedisCache::new(redis_config, Arc::clone(&metrics)).await {
                Ok(cache) => Some(CacheLayer::Redis(cache)),
                Err(e) => {
                    tracing::warn!("Failed to initialize Redis cache: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            l1,
            l2,
            l3: l3_repo,
            config,
            metrics,
        })
    }

    /// Get a value from cache with fallback to compute function.
    pub async fn get_or_compute<T, F, Fut>(
        &self,
        key: &str,
        compute: F,
        scenario: &str,
        user_id: Option<i32>,
        profile_id: Option<&str>,
    ) -> Result<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T>> + Send,
    {
        // Try L1 cache
        if let Some(ref l1) = self.l1 {
            if let Some(value) = l1.get::<T>(key).await? {
                return Ok(value);
            }
        }

        // Try L2 cache
        if let Some(ref l2) = self.l2 {
            if let Some(value) = l2.get::<T>(key).await? {
                // Backfill L1
                let _ = self.set_with_ttl(key, &value, self.config.l1_ttl, scenario, user_id, profile_id).await;
                return Ok(value);
            }
        }

        // Try L3 cache (Postgres)
        if let Some(ref l3) = self.l3 {
            if let Ok(Some(entry)) = l3.get(key).await {
                let value: T = serde_json::from_value(entry.recommendations)
                    .context("Failed to deserialize L3 cache entry")?;
                
                // Backfill L2 & L1
                let _ = self.set_with_ttl(key, &value, self.config.l2_ttl, scenario, user_id, profile_id).await;
                let _ = self.set_with_ttl(key, &value, self.config.l1_ttl, scenario, user_id, profile_id).await;
                return Ok(value);
            }
        }

        // Compute value
        let value = compute().await?;

        // Store in all caches
        self.set(key, &value, scenario, user_id, profile_id).await?;
        Ok(value)
    }


    /// Get a value from cache only (no compute).
    pub async fn get<T>(
        &self, 
        key: &str, 
        scenario: &str, 
        user_id: Option<i32>,
        profile_id: Option<&str>,
    ) -> Result<Option<T>>
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    {
        // Try L1
        if let Some(ref l1) = self.l1 {
            if let Some(value) = l1.get::<T>(key).await? {
                return Ok(Some(value));
            }
        }

        // Try L2
        if let Some(ref l2) = self.l2 {
            if let Some(value) = l2.get::<T>(key).await? {
                // Backfill L1
                let _ = self.set_with_ttl(key, &value, self.config.l1_ttl, scenario, user_id, profile_id).await;
                return Ok(Some(value));
            }
        }

        // Try L3
        if let Some(ref l3) = self.l3 {
            if let Ok(Some(entry)) = l3.get(key).await {
                let value: T = serde_json::from_value(entry.recommendations)?;
                
                // Backfill L2 & L1
                let _ = self.set_with_ttl(key, &value, self.config.l2_ttl, scenario, user_id, profile_id).await;
                let _ = self.set_with_ttl(key, &value, self.config.l1_ttl, scenario, user_id, profile_id).await;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Set a value in all cache tiers with a specific TTL.
    pub async fn set_with_ttl<T>(
        &self, 
        key: &str, 
        value: &T, 
        ttl: std::time::Duration,
        scenario: &str,
        user_id: Option<i32>,
        profile_id: Option<&str>,
    ) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        if let Some(ref l3) = self.l3 {
            let json_val = serde_json::to_value(value)?;
            let pid = profile_id.map(|s| s.to_string());
            let _ = l3.set(key, scenario, user_id, pid, None, json_val, ttl.as_secs() as i32).await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.set(key, value, ttl).await;
        }
        if let Some(ref l1) = self.l1 {
            let _ = l1.set(key, value, ttl).await;
        }
        Ok(())
    }

    /// Set a value in all cache tiers using default TTLs.
    pub async fn set<T>(
        &self, 
        key: &str, 
        value: &T, 
        scenario: &str, 
        user_id: Option<i32>,
        profile_id: Option<&str>,
    ) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        self.set_with_ttl(key, value, self.config.l2_ttl, scenario, user_id, profile_id).await
    }

    /// Delete a value from all cache tiers.
    pub async fn delete(&self, key: &str) -> Result<()> {
        if let Some(ref l1) = self.l1 {
            let _ = l1.delete(key).await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.delete(key).await;
        }
        if let Some(ref l3) = self.l3 {
            let _ = l3.delete(key).await;
        }
        Ok(())
    }

    /// Delete multiple keys matching a pattern from all cache tiers.
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        if let Some(ref l1) = self.l1 {
            let _ = l1.delete_pattern(pattern).await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.delete_pattern(pattern).await;
        }
        if let Some(ref l3) = self.l3 {
            let _ = l3.delete_pattern(pattern).await;
        }
        Ok(())
    }

    /// Get cache metrics snapshot.
    pub fn metrics(&self) -> CacheMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get raw handle to metrics for recording events.
    pub fn metrics_handle(&self) -> Arc<CacheMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Mark cache entries as stale across all tiers.
    pub async fn mark_stale(
        &self,
        user_id: Option<i32>,
        profile_id: Option<&str>,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> Result<()> {
        // 1. Invalidate L1/L2 via pattern (best effort)
        let key_pattern = if let Some(slug) = scenario_slug {
            if let Some(pid) = profile_id {
                format!("rec:{}:p_{}:*", slug, pid)
            } else if let Some(uid) = user_id {
                format!("rec:{}:u_{}:*", slug, uid)
            } else {
                format!("rec:{}:*", slug)
            }
        } else {
            if let Some(pid) = profile_id {
                format!("rec:*:p_{}:*", pid)
            } else if let Some(uid) = user_id {
                format!("rec:*:u_{}:*", uid)
            } else {
                "rec:*".to_string()
            }
        };

        let _ = self.delete_pattern(&key_pattern).await;
        self.metrics.record_invalidation();

        // 2. Mark L3 cache as stale
        if let Some(ref l3) = self.l3 {
            let _ = l3.mark_stale(user_id, profile_id, scenario_slug, reason).await;
        }

        Ok(())
    }

    /// Clear all cache tiers.
    pub async fn clear(&self) -> Result<()> {
        if let Some(ref l1) = self.l1 {
            let _ = l1.clear().await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.clear().await;
        }
        if let Some(ref l3) = self.l3 {
            let _ = l3.clear().await;
        }
        Ok(())
    }

    /// Close all cache tiers gracefully.
    pub async fn close(&self) -> Result<()> {
        if let Some(ref l1) = self.l1 {
            l1.close().await?;
        }
        if let Some(ref l2) = self.l2 {
            l2.close().await?;
        }
        tracing::info!("Cache tiers closed gracefully");
        Ok(())
    }
}
