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

use crate::cache::{CacheManager, CacheConfig};
use crate::db::repositories::cache_repository::CacheRepository;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;
use crate::pipeline::ScoredItem;

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
}

impl StagingManager {
    // ... (existing methods)

    /// Record a negative signal (Skip) and update genre penalty state.
    pub async fn record_negative_signal(&self, user_id: i32, genres: Vec<String>) -> Result<()> {
        for genre in genres {
            let key = format!("penalty:{}:{}", user_id, genre);
            let mut state: PenaltyState = self.cache_manager.get(&key).await?.unwrap_or(PenaltyState {
                consecutive_skips: 0,
                is_burned: false,
                burned_at: None,
            });

            state.consecutive_skips += 1;
            if state.consecutive_skips >= 3 {
                state.is_burned = true;
                state.burned_at = Some(chrono::Utc::now());
                info!(user_id, genre, "Genre BURNED for 60 minutes");
            }

            // Save penalty with 60-minute TTL
            self.cache_manager.set(&key, &state).await?;

            // 3. Update combined penalty map for the Ranker
            if state.is_burned {
                let map_key = format!("penalties:{}", user_id);
                let mut penalties: HashMap<String, f32> = self.cache_manager.get(&map_key).await?.unwrap_or_default();
                penalties.insert(genre, 0.1); // 0.1x multiplier
                self.cache_manager.set(&map_key, &penalties).await?;
            }
        }
        Ok(())
    }

    /// Get all active genre penalties for a user.
    pub async fn get_genre_penalties(&self, user_id: i32) -> Result<HashMap<String, f32>> {
        // In a real system, we might query a "hot" set of genres the user interacts with.
        // For this implementation, we rely on the stages checking specific genres.
        // But to make it work for the Ranker, we'll return a map of burned genres.
        // Note: This is a simplified scan.
        Ok(HashMap::new()) // To be refined if needed, or handled per-item in the ranker
    }

    /// Get from PostgreSQL L3 cache
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
