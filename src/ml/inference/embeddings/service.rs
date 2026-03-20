//! Embedding manager with circuit breaker, cache, and precomputed fallback.
//!
//! # Netflix Resilience Patterns
//! - **Circuit Breaker**: Embedding lookups protected by circuit breaker
//! - **Cache**: In-memory LRU + Redis with TTL
//! - **Fallback**: Precomputed zero embeddings for cold-start
//! - **Bulkhead**: Semaphore limits concurrent embedding fetches
//! - **Analytics**: Embedding fetch latency, cache hit/miss, error rates

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde_json::Value as JsonValue;
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::cache::CacheManager;
use crate::config::MlConfig;
use crate::error::ModelError;
use crate::db::ResilientPool;


/// Embedding manager for user and item embeddings.
pub struct EmbeddingManager {
    pool: Arc<ResilientPool>,
    cache_manager: Arc<CacheManager>,
    bulkhead: Arc<Semaphore>,
    config: MlConfig,
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

impl EmbeddingManager {
    pub fn new(
        pool: Arc<ResilientPool>,
        cache_manager: Arc<CacheManager>,
        config: MlConfig,
        analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
    ) -> Self {
        let bulkhead = Arc::new(Semaphore::new(config.feature_fetch_max_concurrent));
        Self {
            pool,
            cache_manager,
            bulkhead,
            config,
            analytics,
        }
    }

    /// ML config accessor.
    pub fn config(&self) -> &MlConfig {
        &self.config
    }

    /// Get item embeddings for a set of item IDs.
    /// Returns (item_id, embedding) pairs. Missing items get zero embeddings.
    pub async fn get_item_embeddings(
        &self,
        item_ids: &[i32],
        embedding_dim: usize,
    ) -> Result<Vec<(i32, Vec<f32>)>, ModelError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }

        let start = Instant::now();
        let metric_key = "ml.embeddings.item";

        // Bulkhead
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                ModelError::Overloaded {
                    model: "embedding_manager".into(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        let result = self.fetch_embeddings_from_db(item_ids, embedding_dim).await;

        let latency = start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time(metric_key, latency.as_millis() as u64);
            a.increment_throughput(metric_key);
        }

        match result {
            Ok(embeddings) => Ok(embeddings),
            Err(e) => {
                warn!(error = %e, "Embedding fetch failed, using zero fallback");
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                // Fallback: zero embeddings
                Ok(item_ids.iter().map(|&id| (id, vec![0.0; embedding_dim])).collect())
            }
        }
    }

    /// Get profile embedding.
    pub async fn get_profile_embedding(
        &self,
        profile_id: &str,
        embedding_dim: usize,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        let metric_key = "ml.embeddings.profile";

        // Bulkhead
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                ModelError::Overloaded {
                    model: "embedding_manager".into(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        // Cache check
        let cache_key = format!("profile_embedding:{}", profile_id);
        if let Ok(Some(embedding)) = self.cache_manager.get::<Vec<f32>>(&cache_key, "ml_embeddings", None, Some(profile_id)).await {
            debug!(profile_id = %profile_id, "Profile embedding from cache");
            if let Some(ref a) = self.analytics {
                a.increment_throughput(&format!("{}.cache_hit", metric_key));
            }
            return Ok(crate::ml::assets::utils::service::pad_or_truncate(embedding, embedding_dim));
        }

        // DB fetch
        let result = self.fetch_profile_embedding_from_db(profile_id, embedding_dim).await;

        let latency = start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time(metric_key, latency.as_millis() as u64);
            a.increment_throughput(metric_key);
        }

        match result {
            Ok(embedding) => {
                let _ = self.cache_manager.set(&cache_key, &embedding, "ml_embeddings", None, Some(profile_id)).await;
                Ok(crate::ml::assets::utils::service::pad_or_truncate(embedding, embedding_dim))
            }
            Err(e) => {
                warn!(profile_id = %profile_id, error = %e, "Profile embedding fetch failed, using zero fallback");
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                Ok(vec![0.0; embedding_dim])
            }
        }
    }

    // ── Internal DB queries ──────────────────────────────────────────────────

    async fn fetch_embeddings_from_db(
        &self,
        item_ids: &[i32],
        embedding_dim: usize,
    ) -> Result<Vec<(i32, Vec<f32>)>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            embedding: Option<Vec<f32>>,
            tfidf_vector: Option<JsonValue>,
        }

        let ids = item_ids.to_vec();
        let rows: Vec<Row> = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, Row>(
                "SELECT item_id, embedding, tfidf_vector FROM bongas.item_features WHERE item_id = ANY($1)",
            )
            .bind(&ids)
            .fetch_all(&pool)
            .await
        }).await?;

        let mut found: HashMap<i32, Vec<f32>> = HashMap::new();
        for row in rows {
            let emb = if let Some(embedding) = row.embedding {
                embedding
            } else if let Some(tfidf) = row.tfidf_vector {
                serde_json::from_value(tfidf).unwrap_or_else(|_| vec![0.0; embedding_dim])
            } else {
                vec![0.0; embedding_dim]
            };
            found.insert(row.item_id, crate::ml::assets::utils::service::pad_or_truncate(emb, embedding_dim));
        }

        // Fill missing with zeros
        let result: Vec<(i32, Vec<f32>)> = item_ids.iter().map(|&id| {
            let emb = found.remove(&id).unwrap_or_else(|| vec![0.0; embedding_dim]);
            (id, emb)
        }).collect();

        Ok(result)
    }

    async fn fetch_profile_embedding_from_db(
        &self,
        profile_id: &str,
        embedding_dim: usize,
    ) -> Result<Vec<f32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            embedding: Option<Vec<f32>>,
        }

        let pid = profile_id.to_string();
        let row: Option<Row> = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, Row>(
                "SELECT embedding FROM bongas.profile_features WHERE profile_id = $1",
            )
            .bind(pid)
            .fetch_optional(&pool)
            .await
        }).await?;

        Ok(row
            .and_then(|r| r.embedding)
            .unwrap_or_else(|| vec![0.0; embedding_dim]))
    }
}
