//! Centralized feature store with circuit breaker, cache, fallback, and analytics.
//!
//! # Netflix Resilience Patterns
//! - **Circuit Breaker**: DB/cache queries protected by resilient pool
//! - **Bulkhead**: Semaphore limits concurrent feature fetches
//! - **Cache**: L1 in-memory + L2 Redis with configurable TTL
//! - **Fallback**: Cold-start defaults when features are unavailable
//! - **Analytics**: Feature fetch latency, cache hit/miss, error rates

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


/// Centralized feature store for user and item features.
pub struct FeatureStore {
    pool: Arc<ResilientPool>,
    cache_manager: Arc<CacheManager>,
    bulkhead: Arc<Semaphore>,
    config: MlConfig,
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

impl FeatureStore {
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

    /// Get profile features with cache → DB → cold-start fallback chain.
    pub async fn get_profile_features(
        &self,
        profile_id: &str,
        feature_dim: usize,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        let metric_key = "ml.feature_store.profile";

        // Bulkhead
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                ModelError::Overloaded {
                    model: "feature_store".into(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        // L1: Cache
        let cache_key = format!("profile_features:{}", profile_id);
        if let Ok(Some(features)) = self.cache_manager.get::<Vec<f32>>(&cache_key).await {
            debug!(profile_id = %profile_id, "Profile features from cache");
            if let Some(ref a) = self.analytics {
                a.increment_throughput(&format!("{}.cache_hit", metric_key));
            }
            return Ok(crate::ml::assets::utils::service::pad_or_truncate(features, feature_dim));
        }

        // L2: Database
        let features = self.fetch_profile_features_from_db(profile_id, feature_dim).await;

        let latency = start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time(metric_key, latency.as_millis() as u64);
            a.increment_throughput(metric_key);
        }

        match features {
            Ok(feats) => {
                // Write-back to cache (fire-and-forget)
                let _ = self.cache_manager.set(&cache_key, &feats).await;
                Ok(crate::ml::assets::utils::service::pad_or_truncate(feats, feature_dim))
            }
            Err(e) => {
                warn!(profile_id = %profile_id, error = %e, "Feature fetch failed, using cold-start defaults");
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                // Fallback: cold-start zero vector
                Ok(vec![0.0; feature_dim])
            }
        }
    }

    /// Get genre affinities for a visitor.
    pub async fn get_visitor_affinities(&self, visitor_id: &str) -> Result<HashMap<String, f32>> {
        let visitor_id = visitor_id.to_string();
        
        let row: Option<JsonValue> = self.pool.execute(|pool| async move {
            sqlx::query_scalar(
                "SELECT genre_affinity FROM visitor_features WHERE visitor_id = $1"
            )
            .bind(visitor_id)
            .fetch_optional(&pool)
            .await
        }).await?;

        match row {
            Some(json) => {
                let affinities: HashMap<String, f32> = serde_json::from_value(json).unwrap_or_default();
                Ok(affinities)
            }
            None => Ok(HashMap::new())
        }
    }

    /// Get item features for multiple items with cache → DB → fallback chain.
    pub async fn get_item_features(
        &self,
        item_ids: &[i32],
        feature_dim: usize,
    ) -> Result<HashMap<i32, Vec<f32>>, ModelError> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let start = Instant::now();
        let metric_key = "ml.feature_store.item";

        // Bulkhead
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                ModelError::Overloaded {
                    model: "feature_store".into(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        let result = self.fetch_item_features_from_db(item_ids, feature_dim).await;

        let latency = start.elapsed();
        if let Some(ref a) = self.analytics {
            a.record_response_time(metric_key, latency.as_millis() as u64);
            a.increment_throughput(metric_key);
        }

        match result {
            Ok(map) => Ok(map),
            Err(e) => {
                warn!(error = %e, "Item feature fetch failed, using zero defaults");
                if let Some(ref a) = self.analytics {
                    a.increment_error(metric_key);
                }
                // Fallback: zero vectors for all items
                Ok(item_ids.iter().map(|&id| (id, vec![0.0; feature_dim])).collect())
            }
        }
    }

    // ── DB Queries ────────────────────────────────────────────────────────────

    async fn fetch_profile_features_from_db(
        &self,
        profile_id: &str,
        feature_dim: usize,
    ) -> Result<Vec<f32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            genre_affinity: Option<JsonValue>,
            embedding: Option<Vec<f32>>,
            total_watch_time_minutes: Option<i32>,
            avg_completion_rate: Option<f32>,
        }

        let pid = profile_id.to_string();
        let pid_log = profile_id.to_string();
        let row: Option<Row> = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT genre_affinity, embedding, total_watch_time_minutes, avg_completion_rate
                FROM profile_features
                WHERE profile_id = $1
                "#,
            )
            .bind(pid)
            .fetch_optional(&pool)
            .await
        }).await?;

        let features = match row {
            Some(row) => {
                if let Some(embedding) = row.embedding {
                    embedding
                } else {
                    let mut features = Vec::new();
                    if let Some(genre_affinity) = row.genre_affinity {
                        if let Ok(affinity) = serde_json::from_value::<Vec<f32>>(genre_affinity) {
                            features.extend(affinity);
                        }
                    }
                    features.push(row.total_watch_time_minutes.unwrap_or(0) as f32 / 10000.0);
                    features.push(row.avg_completion_rate.unwrap_or(0.0));
                    features
                }
            }
            None => {
                warn!(profile_id = %pid_log, "No profile features found, using cold start defaults");
                vec![0.0; feature_dim]
            }
        };

        Ok(features)
    }

    async fn fetch_item_features_from_db(
        &self,
        item_ids: &[i32],
        feature_dim: usize,
    ) -> Result<HashMap<i32, Vec<f32>>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            embedding: Option<Vec<f32>>,
            tfidf_vector: Option<JsonValue>,
            view_count: Option<i32>,
            trending_score: Option<f32>,
            completion_rate: Option<f32>,
        }

        let ids = item_ids.to_vec();
        let rows: Vec<Row> = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT item_id, embedding, tfidf_vector, view_count, trending_score, completion_rate
                FROM item_features
                WHERE item_id = ANY($1)
                "#,
            )
            .bind(&ids)
            .fetch_all(&pool)
            .await
        }).await?;

        let mut map = HashMap::new();
        for row in rows {
            let features = if let Some(embedding) = row.embedding {
                embedding
            } else {
                let mut features = Vec::new();
                if let Some(tfidf) = row.tfidf_vector {
                    if let Ok(vec) = serde_json::from_value::<Vec<f32>>(tfidf) {
                        features.extend(vec);
                    }
                }
                features.push(row.view_count.unwrap_or(0) as f32 / 100000.0);
                features.push(row.trending_score.unwrap_or(0.0));
                features.push(row.completion_rate.unwrap_or(0.0));
                features
            };
            map.insert(row.item_id, crate::ml::assets::utils::service::pad_or_truncate(features, feature_dim));
        }

        // Fill missing items with zero vectors
        for &id in item_ids {
            map.entry(id).or_insert_with(|| vec![0.0; feature_dim]);
        }

        Ok(map)
    }

    pub fn repository(&self) -> Arc<crate::db::repositories::feature_repository::service::FeatureRepository> {
        use crate::resilience::{MetricsRegistry, ResilienceMetricsConfig, ResilienceMetricsCollector};
        Arc::new(crate::db::repositories::feature_repository::service::FeatureRepository::new(
            self.pool.clone(),
            Arc::new(ResilienceMetricsCollector::new(
                Arc::new(MetricsRegistry::new(ResilienceMetricsConfig::default()))
            )),
        ))
    }
}
