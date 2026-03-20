//! Cache repository with Netflix-grade resilience patterns.
//!
//! Provides L2 (PostgreSQL) cache access for recommendations with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value as JsonValue;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::RecommendationCacheL2;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Payload for saving recommendations to the L2 cache.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntryPayload {
    pub cache_key: String,
    pub scenario_slug: String,
    pub user_id: Option<i32>,
    pub profile_id: Option<String>,
    pub context_hash: Option<String>,
    pub recommendations: JsonValue,
    pub ttl_seconds: i32,
}

/// Repository for L2 recommendation cache with resilience patterns.
#[derive(Clone)]
pub struct CacheRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl CacheRepository {
    /// Create a new repository with resilient pool and metrics collector.
    pub fn new(
        pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            pool,
            metrics_collector,
        }
    }

    /// Access the underlying resilient pool.
    pub fn pool(&self) -> Arc<ResilientPool> {
        self.pool.clone()
    }

    /// Get cached recommendations from L2 by cache key.
    pub async fn get(&self, cache_key: &str) -> AppResult<Option<RecommendationCacheL2>> {
        let start_time = std::time::Instant::now();
        let key = cache_key.to_string();
        
        let result: AppResult<Option<RecommendationCacheL2>> = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, RecommendationCacheL2>(
                r#"
                SELECT * FROM bongas.recommendation_cache_l2
                WHERE cache_key = $1
                    AND expires_at > NOW()
                    AND is_stale = false
                "#,
            )
            .bind(&key)
            .fetch_optional(&pool)
            .await
        }).await;

        let duration = start_time.elapsed();
        
        match result {
            Ok(entry) => {
                // Record successful cache operation
                let metrics = self.metrics_collector.registry().get_or_create("cache_l2");
                metrics.latency.record_duration(duration);
                metrics.successes.increment();
                
                // Update hit count if found (fire and forget, don't fail on this)
                if entry.is_some() {
                    let key = cache_key.to_string();
                    let pool = self.pool.clone();
                    tokio::spawn(async move {
                        let _ = pool.execute(|pool| async move {
                            sqlx::query(
                                "UPDATE bongas.recommendation_cache_l2 SET hit_count = hit_count + 1, last_hit_at = NOW() WHERE cache_key = $1",
                            )
                            .bind(&key)
                            .execute(&pool)
                            .await
                        }).await;
                    });
                }

                Ok(entry)
            }
            Err(e) => {
                // Record cache failure
                let metrics = self.metrics_collector.registry().get_or_create("cache_l2");
                metrics.latency.record_duration(duration);
                metrics.failures.increment();
                
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch cache entry: {}", e),
                    source: None,
                }))
            }
        }
    }

    /// Save recommendations to L2 cache.
    pub async fn set(
        &self,
        payload: CacheEntryPayload,
    ) -> AppResult<()> {
        let expires_at = Utc::now() + Duration::seconds(payload.ttl_seconds as i64);

        self.pool.execute(move |pool| {
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO bongas.recommendation_cache_l2 (
                        cache_key, scenario_slug, user_id, profile_id, context_hash,
                        recommendations, expires_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (cache_key) DO UPDATE SET
                        recommendations = EXCLUDED.recommendations,
                        cached_at = NOW(),
                        expires_at = EXCLUDED.expires_at,
                        is_stale = false,
                        staleness_reason = NULL
                    "#,
                )
                .bind(payload.cache_key)
                .bind(payload.scenario_slug)
                .bind(payload.user_id)
                .bind(payload.profile_id)
                .bind(payload.context_hash)
                .bind(payload.recommendations)
                .bind(expires_at)
                .execute(&pool)
                .await
            }
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to set cache entry: {}", e),
                source: None,
            })
        })
        .map(|_| ())
    }

    /// Mark cache entries as stale for a user/profile.
    pub async fn mark_stale(
        &self,
        user_id: Option<i32>,
        profile_id: Option<&str>,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> AppResult<u64> {
        let scenario_slug = scenario_slug.map(|s| s.to_string());
        let profile_id = profile_id.map(|s| s.to_string());
        let reason = reason.to_string();

        self.pool.execute(|pool| {
            let scenario_slug = scenario_slug.clone();
            let profile_id = profile_id.clone();
            let reason = reason.clone();
            async move {
                let mut query_str = "UPDATE bongas.recommendation_cache_l2 SET is_stale = true, staleness_reason = $1 WHERE 1=1".to_string();
                let mut arg_idx = 2;
                
                if user_id.is_some() {
                    query_str.push_str(&format!(" AND user_id = ${}", arg_idx));
                    arg_idx += 1;
                }
                if profile_id.is_some() {
                    query_str.push_str(&format!(" AND profile_id = ${}", arg_idx));
                    arg_idx += 1;
                }
                if scenario_slug.is_some() {
                    query_str.push_str(&format!(" AND scenario_slug = ${}", arg_idx));
                }

                let mut q = sqlx::query(&query_str).bind(&reason);
                if let Some(uid) = user_id { q = q.bind(uid); }
                if let Some(pid) = profile_id { q = q.bind(pid); }
                if let Some(slug) = scenario_slug { q = q.bind(slug); }

                q.execute(&pool).await
            }
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to mark cache stale: {}", e),
                source: None,
            })
        })
        .map(|r| r.rows_affected())
    }

    /// Delete a specific cache entry.
    pub async fn delete(&self, cache_key: &str) -> AppResult<()> {
        let key = cache_key.to_string();
        self.pool.execute(|pool| {
            let key = key.clone();
            async move {
                sqlx::query("DELETE FROM bongas.recommendation_cache_l2 WHERE cache_key = $1")
                    .bind(key)
                    .execute(&pool)
                    .await
            }
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to delete cache entry: {}", e),
                source: None,
            })
        })
        .map(|_| ())
    }

    /// Delete multiple cache entries matching a prefix/pattern.
    pub async fn delete_pattern(&self, pattern: &str) -> AppResult<()> {
        // Convert L1 glob-like pattern to Postgres LIKE pattern
        let pg_pattern = pattern.replace("*", "%").replace("?", "_");
        
        self.pool.execute(|pool| {
            let p = pg_pattern.clone();
            async move {
                sqlx::query("DELETE FROM bongas.recommendation_cache_l2 WHERE cache_key LIKE $1")
                    .bind(p)
                    .execute(&pool)
                    .await
            }
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to delete cache pattern: {}", e),
                source: None,
            })
        })
        .map(|_| ())
    }

    /// Clear all entries in the cache table.
    pub async fn clear(&self) -> AppResult<()> {
        self.pool.execute(|pool| async move {
            sqlx::query("DELETE FROM bongas.recommendation_cache_l2")
                .execute(&pool)
                .await
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to clear cache: {}", e),
                source: None,
            })
        })
        .map(|_| ())
    }

    /// Delete expired cache entries (called by cleanup worker).
    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        self.pool.execute(|pool| async move {
            sqlx::query(
                "DELETE FROM bongas.recommendation_cache_l2 WHERE expires_at < NOW() OR is_stale = true",
            )
            .execute(&pool)
            .await
        })
        .await
        .map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to cleanup expired cache: {}", e),
                source: None,
            })
        })
        .map(|r| r.rows_affected())
    }
}
