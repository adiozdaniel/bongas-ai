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
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::cache::CacheManager;
use crate::config::MlConfig;
use crate::error::ModelError;


/// Embedding manager for user and item embeddings.
pub struct EmbeddingManager {
    db_pool: Arc<PgPool>,
    cache_manager: Arc<CacheManager>,
    bulkhead: Arc<Semaphore>,
    config: MlConfig,
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

impl EmbeddingManager {
    pub fn new(
        db_pool: Arc<PgPool>,
        cache_manager: Arc<CacheManager>,
        config: MlConfig,
        analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
    ) -> Self {
        let bulkhead = Arc::new(Semaphore::new(config.feature_fetch_max_concurrent));
        Self {
            db_pool,
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

    /// Get user embedding.
    pub async fn get_user_embedding(
        &self,
        user_id: i32,
        embedding_dim: usize,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        let metric_key = "ml.embeddings.user";

        // Bulkhead
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                ModelError::Overloaded {
                    model: "embedding_manager".into(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        // Cache check
        let cache_key = format!("user_embedding:{}", user_id);
        if let Ok(Some(embedding)) = self.cache_manager.get::<Vec<f32>>(&cache_key).await {
            debug!(user_id = user_id, "User embedding from cache");
            if let Some(ref a) = self.analytics {
                a.increment_throughput(&format!("{}.cache_hit", metric_key));
            }
            return Ok(crate::ml::utils::pad_or_truncate(embedding, embedding_dim));
        }

        // DB fetch
        let result = self.fetch_user_embedding_from_db(user_id, embedding_dim).await;

        let latency = start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time(metric_key, latency.as_millis() as u64);
            a.increment_throughput(metric_key);
        }

        match result {
            Ok(embedding) => {
                let _ = self.cache_manager.set(&cache_key, &embedding).await;
                Ok(crate::ml::utils::pad_or_truncate(embedding, embedding_dim))
            }
            Err(e) => {
                warn!(user_id = user_id, error = %e, "User embedding fetch failed, using zero fallback");
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                Ok(vec![0.0; embedding_dim])
            }
        }
    }

    /// Compute cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 { dot / (norm_a * norm_b) } else { 0.0 }
    }

    /// Compute dot product between two vectors.
    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Compute euclidean similarity (1 / (1 + distance)).
    pub fn euclidean_similarity(a: &[f32], b: &[f32]) -> f32 {
        let distance: f32 = a.iter().zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();
        1.0 / (1.0 + distance)
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

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT item_id, embedding, tfidf_vector FROM item_features WHERE item_id = ANY($1)",
        )
        .bind(item_ids)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut found: HashMap<i32, Vec<f32>> = HashMap::new();
        for row in rows {
            let emb = if let Some(embedding) = row.embedding {
                embedding
            } else if let Some(tfidf) = row.tfidf_vector {
                serde_json::from_value(tfidf).unwrap_or_else(|_| vec![0.0; embedding_dim])
            } else {
                vec![0.0; embedding_dim]
            };
            found.insert(row.item_id, crate::ml::utils::pad_or_truncate(emb, embedding_dim));
        }

        // Fill missing with zeros
        let result: Vec<(i32, Vec<f32>)> = item_ids.iter().map(|&id| {
            let emb = found.remove(&id).unwrap_or_else(|| vec![0.0; embedding_dim]);
            (id, emb)
        }).collect();

        Ok(result)
    }

    async fn fetch_user_embedding_from_db(
        &self,
        user_id: i32,
        embedding_dim: usize,
    ) -> Result<Vec<f32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            embedding: Option<Vec<f32>>,
        }

        let row: Option<Row> = sqlx::query_as(
            "SELECT embedding FROM user_features WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        Ok(row
            .and_then(|r| r.embedding)
            .unwrap_or_else(|| vec![0.0; embedding_dim]))
    }


}
