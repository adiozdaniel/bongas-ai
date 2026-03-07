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
        
        let result = self.pool.execute(|pool| async move {
            sqlx::query_as::<_, RecommendationCacheL2>(
                r#"
                SELECT * FROM recommendation_cache_l2
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
                                "UPDATE recommendation_cache_l2 SET hit_count = hit_count + 1, last_hit_at = NOW() WHERE cache_key = $1",
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
        cache_key: &str,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: Option<&str>,
        recommendations: JsonValue,
        ttl_seconds: i32,
    ) -> AppResult<()> {
        let cache_key = cache_key.to_string();
        let scenario_slug = scenario_slug.to_string();
        let context_hash = context_hash.map(|s| s.to_string());
        let expires_at = Utc::now() + Duration::seconds(ttl_seconds as i64);

        self.pool.execute(|pool| {
            let cache_key = cache_key.clone();
            let scenario_slug = scenario_slug.clone();
            let context_hash = context_hash.clone();
            let recommendations = recommendations.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO recommendation_cache_l2 (
                        cache_key, scenario_slug, user_id, context_hash,
                        recommendations, expires_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (cache_key) DO UPDATE SET
                        recommendations = EXCLUDED.recommendations,
                        cached_at = NOW(),
                        expires_at = EXCLUDED.expires_at,
                        is_stale = false,
                        staleness_reason = NULL
                    "#,
                )
                .bind(&cache_key)
                .bind(&scenario_slug)
                .bind(user_id)
                .bind(&context_hash)
                .bind(&recommendations)
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

    /// Mark cache entries as stale for a user.
    pub async fn mark_stale(
        &self,
        user_id: i32,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> AppResult<u64> {
        let scenario_slug = scenario_slug.map(|s| s.to_string());
        let reason = reason.to_string();

        self.pool.execute(|pool| {
            let scenario_slug = scenario_slug.clone();
            let reason = reason.clone();
            async move {
                if let Some(slug) = scenario_slug {
                    sqlx::query(
                        "UPDATE recommendation_cache_l2 SET is_stale = true, staleness_reason = $3 WHERE user_id = $1 AND scenario_slug = $2",
                    )
                    .bind(user_id)
                    .bind(&slug)
                    .bind(&reason)
                    .execute(&pool)
                    .await
                } else {
                    sqlx::query(
                        "UPDATE recommendation_cache_l2 SET is_stale = true, staleness_reason = $2 WHERE user_id = $1",
                    )
                    .bind(user_id)
                    .bind(&reason)
                    .execute(&pool)
                    .await
                }
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
                sqlx::query("DELETE FROM recommendation_cache_l2 WHERE cache_key = $1")
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
                sqlx::query("DELETE FROM recommendation_cache_l2 WHERE cache_key LIKE $1")
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

    /// Delete expired cache entries (called by cleanup worker).
    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        self.pool.execute(|pool| async move {
            sqlx::query(
                "DELETE FROM recommendation_cache_l2 WHERE expires_at < NOW() OR is_stale = true",
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
