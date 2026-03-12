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

use crate::cache::CacheManager;
use crate::db::CacheRepository;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;
use crate::pipeline::types::models::ScoredItem;

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PenaltyState {
    pub consecutive_skips: u32,
    pub is_burned: bool,
    pub burned_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct StagingManager {
    cache_manager: Arc<CacheManager>,
    cache_repo: CacheRepository,
    // Fix #17: Per-user locks for atomic penalty updates
    penalty_locks: dashmap::DashMap<i32, Arc<tokio::sync::Mutex<()>>>,
}

impl StagingManager {
    pub fn new(
        cache_manager: Arc<CacheManager>,
        pool: Arc<ResilientPool>,
        metrics: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            cache_manager,
            cache_repo: CacheRepository::new(pool, metrics),
            penalty_locks: dashmap::DashMap::new(),
        }
    }

    /// Get recommendations from cache tiers (L1 -> L2 -> L3)
    pub async fn get_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        profile_id: Option<&str>,
        context_hash: &str,
    ) -> Result<Option<Vec<ScoredItem>>> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, profile_id, context_hash);
        let start_time = Instant::now();

        // Try L1 (LRU) and L2 (Redis) via CacheManager
        if let Some(items) = self.cache_manager.get::<Vec<ScoredItem>>(&cache_key, scenario_slug, user_id, profile_id).await? {
            let duration = start_time.elapsed();
            info!(
                cache_key = %cache_key,
                duration_ms = duration.as_millis(),
                "Cache hit (L1/L2)"
            );
            return Ok(Some(items));
        }

        // Try L3 (PostgreSQL)
        if let Some(items) = self.get_from_l3(&cache_key).await? {
            let duration = start_time.elapsed();
            info!(
                cache_key = %cache_key,
                duration_ms = duration.as_millis(),
                "L3 cache hit (PostgreSQL)"
            );

            // Promote to L1/L2
            let _ = self.cache_manager.set(&cache_key, &items, scenario_slug, user_id, profile_id).await;

            return Ok(Some(items));
        }

        let duration = start_time.elapsed();
        debug!(
            cache_key = %cache_key,
            duration_ms = duration.as_millis(),
            "Cache miss"
        );
        Ok(None)
    }

    /// Save recommendations to all cache tiers
    pub async fn save_cached(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        profile_id: Option<&str>,
        context_hash: &str,
        items: &[ScoredItem],
        ttl_seconds: i32,
    ) -> Result<()> {
        let cache_key = Self::build_cache_key(scenario_slug, user_id, profile_id, context_hash);

        // Save to L1/L2 via CacheManager
        self.cache_manager.set(&cache_key, &items.to_vec(), scenario_slug, user_id, profile_id).await?;

        // Save to L3 (PostgreSQL) with longer TTL (12x)
        let l3_ttl = ttl_seconds * 12;
        let items_json = serde_json::to_value(items)?;
        let pid_owned = profile_id.map(|s| s.to_string());
        self.cache_repo.set(crate::db::repositories::cache_repository::service::CacheEntryPayload {
            cache_key: cache_key.clone(),
            scenario_slug: scenario_slug.to_string(),
            user_id,
            profile_id: pid_owned,
            context_hash: Some(context_hash.to_string()),
            recommendations: items_json,
            ttl_seconds: l3_ttl,
        }).await?;

        info!(
            cache_key = %cache_key,
            l3_ttl = l3_ttl,
            "Saved to all cache tiers"
        );

        Ok(())
    }

    /// Mark cache entries as stale across all tiers (L1, L2, L3).
    pub async fn mark_stale(
        &self,
        user_id: i32,
        profile_id: Option<&str>,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> Result<()> {
        // Use coordinated CacheManager invalidation
        self.cache_manager.mark_stale(Some(user_id), profile_id, scenario_slug, reason).await?;

        info!(
            user_id = user_id,
            profile_id = ?profile_id,
            scenario_slug = ?scenario_slug,
            reason = reason,
            "Coordinated cache invalidation complete"
        );

        Ok(())
    }

    /// Invalidate cache for a specific scenario + user/profile
    pub async fn invalidate(
        &self,
        scenario_slug: &str,
        user_id: i32,
        profile_id: Option<&str>,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Use coordinated CacheManager invalidation
        self.cache_manager.mark_stale(Some(user_id), profile_id, Some(scenario_slug), "invalidate").await?;

        let duration = start_time.elapsed();

        info!(
            scenario_slug = scenario_slug,
            user_id = user_id,
            profile_id = ?profile_id,
            duration_ms = duration.as_millis(),
            "Invalidated all cache tiers (Coordinated)"
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
            invalidations: metrics.invalidations,
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
    fn build_cache_key(scenario_slug: &str, user_id: Option<i32>, profile_id: Option<&str>, context_hash: &str) -> String {
        let id_part = if let Some(pid) = profile_id {
            format!("p_{}", pid)
        } else {
            format!("u_{}", user_id.map(|id| id.to_string()).unwrap_or_else(|| "anon".to_string()))
        };
        format!("rec:{}:{}:{}", scenario_slug, id_part, context_hash)
    }

    /// Hash context parameters
    pub fn hash_context(params: &JsonValue) -> String {
        let mut hasher = Sha256::new();
        hasher.update(params.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Record a negative signal (Skip) and update genre penalty state.
    pub async fn record_negative_signal(&self, user_id: i32, profile_id: Option<&str>, genres: Vec<String>) -> Result<()> {
        // Fix #17: Acquire per-user lock for atomic update
        let lock = self.penalty_locks.entry(user_id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .value()
            .clone();
        
        let _guard = lock.lock().await;

        for genre in genres {
            let id_part = profile_id.unwrap_or("default");
            let key = format!("penalty:{}:{}:{}", user_id, id_part, genre);
            let mut state: PenaltyState = self.cache_manager.get(&key, "genre_penalty", Some(user_id), profile_id).await?.unwrap_or(PenaltyState {
                consecutive_skips: 0,
                is_burned: false,
                burned_at: None,
            });

            state.consecutive_skips += 1;
            if state.consecutive_skips >= 3 {
                state.is_burned = true;
                state.burned_at = Some(chrono::Utc::now());
                info!(user_id, profile_id = ?profile_id, genre, "Genre BURNED for 60 minutes");
            }

            // Save penalty with 60-minute TTL
            self.cache_manager.set(&key, &state, "genre_penalty", Some(user_id), profile_id).await?;

            // Update combined penalty map for the Ranker
            if state.is_burned {
                let map_key = format!("penalties:{}:{}", user_id, id_part);
                let mut penalties: HashMap<String, f32> = self.cache_manager.get(&map_key, "genre_penalty_map", Some(user_id), profile_id).await?.unwrap_or_default();
                penalties.insert(genre, 0.1); // 0.1x multiplier
                self.cache_manager.set(&map_key, &penalties, "genre_penalty_map", Some(user_id), profile_id).await?;
            }
        }
        Ok(())
    }

    /// Get all active genre penalties for a user/profile.
    pub async fn get_genre_penalties(&self, user_id: i32, profile_id: Option<&str>) -> Result<HashMap<String, f32>> {
        let id_part = profile_id.unwrap_or("default");
        let map_key = format!("penalties:{}:{}", user_id, id_part);
        Ok(self.cache_manager.get(&map_key, "genre_penalty_map", Some(user_id), profile_id).await?.unwrap_or_default())
    }

    /// Fix #M5, N2: Periodically clear unused penalty locks to prevent memory leak
    pub fn cleanup_locks(&self) {
        if self.penalty_locks.len() > 10000 {
            debug!(count = self.penalty_locks.len(), "Surgically cleaning up penalty locks map");
            // Only keep locks that are currently being referenced by other tasks
            self.penalty_locks.retain(|_, lock| Arc::strong_count(lock) > 1);
        }
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
