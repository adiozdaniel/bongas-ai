use anyhow::Result;
use sqlx::PgPool;
use chrono::{Utc, Duration};
use serde_json::Value as JsonValue;
use crate::db::models::RecommendationCacheL2;

#[derive(Debug)]
pub struct CacheRepository {
    pool: PgPool,
}

impl CacheRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get cached recommendations from L2
    pub async fn get(&self, cache_key: &str) -> Result<Option<RecommendationCacheL2>> {
        let entry = sqlx::query_as::<_, RecommendationCacheL2>(
            r#"
            SELECT * FROM recommendation_cache_l2
            WHERE cache_key = $1
                AND expires_at > NOW()
                AND is_stale = false
            "#
        )
        .bind(cache_key)
        .fetch_optional(&self.pool)
        .await?;

        // Update hit count if found
        if entry.is_some() {
            sqlx::query(
                "UPDATE recommendation_cache_l2 SET hit_count = hit_count + 1, last_hit_at = NOW() WHERE cache_key = $1"
            )
            .bind(cache_key)
            .execute(&self.pool)
            .await?;
        }

        Ok(entry)
    }

    /// Save recommendations to L2 cache
    pub async fn set(
        &self,
        cache_key: &str,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_hash: Option<&str>,
        recommendations: JsonValue,
        ttl_seconds: i32,
    ) -> Result<()> {
        let expires_at = Utc::now() + Duration::seconds(ttl_seconds as i64);

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
            "#
        )
        .bind(cache_key)
        .bind(scenario_slug)
        .bind(user_id)
        .bind(context_hash)
        .bind(recommendations)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark cache entries as stale for a user
    pub async fn mark_stale(
        &self,
        user_id: i32,
        scenario_slug: Option<&str>,
        reason: &str,
    ) -> Result<u64> {
        let rows_affected = if let Some(slug) = scenario_slug {
            sqlx::query(
                "UPDATE recommendation_cache_l2 SET is_stale = true, staleness_reason = $3 WHERE user_id = $1 AND scenario_slug = $2"
            )
            .bind(user_id)
            .bind(slug)
            .bind(reason)
            .execute(&self.pool)
            .await?
            .rows_affected()
        } else {
            sqlx::query(
                "UPDATE recommendation_cache_l2 SET is_stale = true, staleness_reason = $2 WHERE user_id = $1"
            )
            .bind(user_id)
            .bind(reason)
            .execute(&self.pool)
            .await?
            .rows_affected()
        };

        Ok(rows_affected)
    }

    /// Delete expired cache entries (called by cleanup worker)
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM recommendation_cache_l2 WHERE expires_at < NOW() OR is_stale = true"
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get cache stats for a scenario
    pub async fn get_cache_stats(&self, scenario_slug: &str) -> Result<CacheStats> {
        let row = sqlx::query_as::<_, CacheStats>(
            r#"
            SELECT
                COUNT(*) as total_entries,
                COUNT(*) FILTER (WHERE is_stale = false AND expires_at > NOW()) as active_entries,
                COUNT(*) FILTER (WHERE is_stale = true) as stale_entries,
                COALESCE(SUM(hit_count), 0) as total_hits
            FROM recommendation_cache_l2
            WHERE scenario_slug = $1
            "#
        )
        .bind(scenario_slug)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: i64,
    pub active_entries: i64,
    pub stale_entries: i64,
    pub total_hits: i64,
}
