//! Unified item feature service for pipeline stages.
//!
//! Provides a single abstraction over item_features queries so that pipeline
//! stages never issue raw SQL. All queries go through the resilient pool with
//! circuit breaker + bulkhead + metrics.
//!
//! # Why this exists
//! Before this service, ~20 pipeline stages each issued their own
//! `sqlx::query_as("SELECT ... FROM item_features WHERE item_id = ANY($1)")`
//! directly on the raw PgPool, bypassing all resilience infrastructure.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::FromRow;

use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;

// ─── Row types (internal) ──────────────────────────────────────────────────

/// Full item features row — superset of all columns pipeline stages may need.
#[derive(Debug, Clone, FromRow)]
pub struct ItemFeatureRow {
    pub item_id: i32,
    // Content metadata
    pub title: Option<String>,
    pub description: Option<String>,
    pub genres: Option<JsonValue>,
    pub tags: Option<JsonValue>,
    pub creators: Option<JsonValue>,
    pub directors: Option<JsonValue>,
    pub studios: Option<JsonValue>,
    pub actors: Option<JsonValue>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub audio_languages: Option<JsonValue>,
    pub subtitle_languages: Option<JsonValue>,
    pub age_rating: Option<String>,
    pub duration_seconds: Option<i32>,
    pub release_year: Option<i32>,
    pub release_date: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub added_date: Option<DateTime<Utc>>,
    pub available_from: Option<DateTime<Utc>>,
    pub available_until: Option<DateTime<Utc>>,
    pub is_active: bool,
    // Quality & Technical
    pub max_resolution: Option<String>,
    pub has_hdr: Option<bool>,
    pub has_dolby_vision: Option<bool>,
    pub has_dolby_atmos: Option<bool>,
    // Content Warnings
    pub is_explicit: Option<bool>,
    pub has_violence: Option<bool>,
    pub has_strong_language: Option<bool>,
    pub has_drug_content: Option<bool>,
    // Country availability
    pub available_countries: Option<JsonValue>,
    pub blocked_countries: Option<JsonValue>,
    // Specialized Tags
    pub seasonal_tags: Option<JsonValue>,
    pub holiday_tags: Option<JsonValue>,
    pub themes: Option<JsonValue>,
    pub is_award_winner: Option<bool>,
    pub required_tier: Option<String>,
    pub is_free: Option<bool>,
    // Scores
    pub view_count: i32,
    pub like_count: i32,
    pub comment_count: Option<i64>,
    pub share_count: Option<i64>,
    pub save_count: Option<i64>,
    pub completion_rate: f32,
    pub trending_score: f32,
    pub popularity_score: Option<f32>,
    pub user_rating: Option<f32>,
    pub user_rating_count: Option<i32>,
    pub critic_rating: Option<f32>,
    pub critic_rating_count: Option<i32>,
    // Embeddings & vectors
    pub embedding: Option<Vec<f32>>,
    pub tfidf_vector: Option<JsonValue>,
}

/// User features row.
#[derive(Debug, Clone, FromRow)]
pub struct UserFeatureRow {
    pub user_id: i32,
    pub genre_affinity: Option<JsonValue>,
    pub disliked_genres: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub favorite_creators: Option<JsonValue>,
    pub preferred_content_type: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

/// User interaction row for "already watched" queries.
#[derive(Debug, Clone, FromRow)]
pub struct WatchedItemRow {
    pub item_id: i32,
}

/// User content preferences row.
#[derive(Debug, Clone, FromRow)]
pub struct UserContentPreferencesRow {
    pub allow_explicit: Option<bool>,
    pub allow_violence: Option<bool>,
    pub allow_language: Option<bool>,
    pub allow_drugs: Option<bool>,
}

/// User profile row (subset).
#[derive(Debug, Clone, FromRow)]
pub struct UserProfileRow {
    pub segment: Option<String>,
    pub subscription_tier: Option<String>,
}

/// Item promotion row.
#[derive(Debug, Clone, FromRow)]
pub struct PromotionRow {
    pub item_id: i32,
    pub promotion_priority: Option<i32>,
    pub target_segments: Option<JsonValue>,
    pub promotion_label: Option<String>,
}

/// User-item personalized score row.
#[derive(Debug, Clone, FromRow)]
pub struct UserItemScoreRow {
    pub item_id: i32,
    pub score: f32,
    pub model_type: String,
}

// ─── Service ───────────────────────────────────────────────────────────────

/// Resilient item feature service used by all pipeline stages.
///
/// Every query goes through `ResilientPool` → circuit breaker + bulkhead + timeout.
pub struct ItemFeatureService {
    pool: Arc<ResilientPool>,
    metrics: Arc<ResilienceMetricsCollector>,
}

impl ItemFeatureService {
    pub fn new(pool: Arc<ResilientPool>, metrics: Arc<ResilienceMetricsCollector>) -> Self {
        Self { pool, metrics }
    }

    // ── Item feature queries ───────────────────────────────────────────

    /// Batch-fetch full item features for a set of item IDs.
    ///
    /// This is the primary method used by filter, boost, diversify, and enrich
    /// stages. Returns a HashMap keyed by item_id for O(1) lookup.
    pub async fn get_item_features_batch(
        &self,
        item_ids: &[i32],
    ) -> Result<HashMap<i32, ItemFeatureRow>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids = item_ids.to_vec();
        let start = std::time::Instant::now();

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(
                    r#"
                    SELECT item_id, title, description, genres, tags, creators,
                           directors, studios, actors,
                           content_type, language, audio_languages, subtitle_languages,
                           age_rating, duration_seconds, release_year, release_date,
                           published_at, added_date, available_from, available_until, is_active,
                           max_resolution, has_hdr, has_dolby_vision, has_dolby_atmos,
                           is_explicit, has_violence, has_strong_language, has_drug_content,
                           available_countries, blocked_countries,
                           seasonal_tags, holiday_tags, themes, is_award_winner,
                           required_tier, is_free,
                           view_count, like_count, comment_count, share_count, save_count,
                           completion_rate, trending_score, popularity_score,
                           user_rating, user_rating_count, critic_rating, critic_rating_count,
                           embedding, tfidf_vector
                    FROM item_features
                    WHERE item_id = ANY($1)
                    "#,
                )
                .bind(&ids)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.batch");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| (r.item_id, r)).collect())
    }

    // ── User queries ──────────────────────────────────────────────────

    /// Get user features for a single user.
    pub async fn get_user_features(&self, user_id: i32) -> Result<Option<UserFeatureRow>> {
        let start = std::time::Instant::now();

        let row: Option<UserFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserFeatureRow>(
                    r#"
                    SELECT user_id, genre_affinity, disliked_genres, total_watch_time_minutes,
                           total_videos_watched, avg_completion_rate,
                           favorite_genres, favorite_creators, preferred_content_type, embedding
                    FROM user_features
                    WHERE user_id = $1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(row)
    }

    /// Get user content preferences.
    pub async fn get_user_content_preferences(
        &self,
        user_id: i32,
    ) -> Result<Option<UserContentPreferencesRow>> {
        let start = std::time::Instant::now();

        let row: Option<UserContentPreferencesRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserContentPreferencesRow>(
                    r#"
                    SELECT allow_explicit, allow_violence, allow_language, allow_drugs
                    FROM user_content_preferences
                    WHERE user_id = $1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user_prefs");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(row)
    }

    /// Get user profile (subset).
    pub async fn get_user_profile(&self, user_id: i32) -> Result<Option<UserProfileRow>> {
        let start = std::time::Instant::now();

        let row: Option<UserProfileRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserProfileRow>(
                    r#"
                    SELECT segment, subscription_tier
                    FROM user_profiles
                    WHERE user_id = $1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user_profile");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(row)
    }

    // ── Interaction queries ────────────────────────────────────────────

    /// Get set of item IDs the user has fully watched (completion > 90%).
    pub async fn get_watched_item_ids(&self, user_id: i32) -> Result<Vec<i32>> {
        let start = std::time::Instant::now();

        let rows: Vec<WatchedItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, WatchedItemRow>(
                    r#"
                    SELECT DISTINCT item_id
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND completion_percentage > 0.9
                    "#,
                )
                .bind(user_id)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.watched");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Get recent watches for a user within a time window.
    pub async fn get_recent_watches(
        &self,
        user_id: i32,
        hours: i32,
        limit: i64,
    ) -> Result<Vec<RecentWatchRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<RecentWatchRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, RecentWatchRow>(
                    r#"
                    SELECT item_id, MAX(created_at) as last_watched
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND created_at >= NOW() - make_interval(hours => $2)
                    GROUP BY item_id
                    ORDER BY last_watched DESC
                    LIMIT $3
                    "#,
                )
                .bind(user_id)
                .bind(hours)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.recent_watches");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Get recent interaction item IDs for a user, limited by count.
    pub async fn get_user_recent_interaction_ids(
        &self,
        user_id: i32,
        limit: i64,
    ) -> Result<Vec<i32>> {
        let start = std::time::Instant::now();

        let rows: Vec<WatchedItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, WatchedItemRow>(
                    r#"
                    SELECT item_id
                    FROM user_interactions
                    WHERE user_id = $1 AND interaction_type = 'view'
                    ORDER BY created_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user_recent_interactions");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    // ── Score & Personalization queries ───────────────────────────────

    /// Get pre-computed personalization scores for a set of items.
    pub async fn get_user_item_scores_batch(
        &self,
        user_id: i32,
        item_ids: &[i32],
        model_type: &str,
    ) -> Result<Vec<UserItemScoreRow>> {
        let start = std::time::Instant::now();
        let ids = item_ids.to_vec();
        let model = model_type.to_string();

        let rows: Vec<UserItemScoreRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserItemScoreRow>(
                    r#"
                    SELECT item_id, score, model_type
                    FROM user_item_scores
                    WHERE user_id = $1
                        AND item_id = ANY($2)
                        AND model_type = $3
                    "#,
                )
                .bind(user_id)
                .bind(&ids)
                .bind(&model)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user_item_scores");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    // ── Promotion queries ─────────────────────────────────────────────

    /// Get active promotions for a set of items.
    pub async fn get_promotions_batch(
        &self,
        item_ids: &[i32],
    ) -> Result<Vec<PromotionRow>> {
        let start = std::time::Instant::now();
        let ids = item_ids.to_vec();
        let now = Utc::now();

        let rows: Vec<PromotionRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, PromotionRow>(
                    r#"
                    SELECT item_id, promotion_priority, target_segments, promotion_label
                    FROM item_promotions
                    WHERE item_id = ANY($1)
                        AND (promotion_start IS NULL OR promotion_start <= $2)
                        AND (promotion_end IS NULL OR promotion_end >= $2)
                        AND is_active = true
                    "#,
                )
                .bind(&ids)
                .bind(now)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.promotions");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    // ── Similarity & Discovery queries ────────────────────────────────

    /// Get candidate item IDs for similarity search, excluding certain IDs and limited by count.
    pub async fn get_candidate_item_ids_for_similarity(
        &self,
        exclude_ids: &[i32],
        limit: i64,
    ) -> Result<Vec<i32>> {
        let start = std::time::Instant::now();

        let rows: Vec<WatchedItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, WatchedItemRow>(
                    r#"
                    SELECT item_id FROM item_features
                    WHERE item_id != ALL($1)
                      AND (embedding IS NOT NULL OR tfidf_vector IS NOT NULL)
                    ORDER BY trending_score DESC NULLS LAST, view_count DESC NULLS LAST
                    LIMIT $2
                    "#,
                )
                .bind(exclude_ids)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.candidate_similarity_ids");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Get user's most recently watched item with a minimum completion rate.
    pub async fn get_recent_watch_with_completion(
        &self,
        user_id: i32,
        min_completion: f32,
    ) -> Result<Option<i32>> {
        let start = std::time::Instant::now();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
        }

        let row: Option<Row> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, Row>(
                    r#"
                    SELECT item_id
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND (completion_rate >= $2 OR completion_percentage >= $2)
                    ORDER BY created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(user_id)
                .bind(min_completion)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.recent_watch_completion");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(row.map(|r| r.item_id))
    }

    /// Find items that overlap with given genres or creators.
    pub async fn get_items_by_overlap(
        &self,
        exclude_item_id: i32,
        genres: &[String],
        creators: &[String],
        limit: i64,
    ) -> Result<Vec<ItemFeatureRow>> {
        let start = std::time::Instant::now();
        let genres_json = serde_json::to_value(genres)?;
        let creators_json = serde_json::to_value(creators)?;

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(
                    r#"
                    SELECT item_id, title, description, genres, tags, creators,
                           content_type, language, audio_languages, subtitle_languages,
                           age_rating, duration_seconds, release_year, published_at, is_active,
                           max_resolution, has_hdr, has_dolby_vision, has_dolby_atmos,
                           is_explicit, has_violence, has_strong_language, has_drug_content,
                           available_countries, blocked_countries,
                           seasonal_tags, holiday_tags, themes,
                           view_count, like_count, comment_count, share_count, save_count,
                           completion_rate, trending_score, popularity_score,
                           embedding, tfidf_vector
                    FROM item_features
                    WHERE item_id != $1
                        AND is_active = true
                        AND (genres ?| $2 OR creators ?| $3)
                    ORDER BY popularity_score DESC NULLS LAST
                    LIMIT $4
                    "#,
                )
                .bind(exclude_item_id)
                .bind(&genres_json)
                .bind(&creators_json)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.overlap");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    // ── Fetch queries (for fetch stages) ───────────────────────────────

    /// Fetch popular content ordered by view count.
    pub async fn get_popular_content(
        &self,
        min_views: i32,
        limit: i64,
    ) -> Result<Vec<PopularItemRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<PopularItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, PopularItemRow>(
                    r#"
                    SELECT item_id, view_count, trending_score, completion_rate
                    FROM item_features
                    WHERE view_count >= $1
                    ORDER BY view_count DESC
                    LIMIT $2
                    "#,
                )
                .bind(min_views)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.popular");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch new releases.
    pub async fn get_new_releases(&self, days: i32, limit: i64) -> Result<Vec<NewReleaseRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<NewReleaseRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, NewReleaseRow>(
                    r#"
                    SELECT item_id, title, published_at, trending_score
                    FROM item_features
                    WHERE published_at >= NOW() - make_interval(days => $1)
                    ORDER BY published_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(days)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.new_releases");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch items by genre using JSONB containment.
    pub async fn get_items_by_genre(
        &self,
        genre_json: JsonValue,
        limit: i64,
    ) -> Result<Vec<ItemFeatureRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(
                    r#"
                    SELECT item_id, title, description, genres, tags, creators,
                           directors, studios, actors,
                           content_type, language, audio_languages, subtitle_languages,
                           age_rating, duration_seconds, release_year, release_date,
                           published_at, added_date, available_from, available_until, is_active,
                           max_resolution, has_hdr, has_dolby_vision, has_dolby_atmos,
                           is_explicit, has_violence, has_strong_language, has_drug_content,
                           available_countries, blocked_countries,
                           seasonal_tags, holiday_tags, themes, is_award_winner,
                           required_tier, is_free,
                           view_count, like_count, comment_count, share_count, save_count,
                           completion_rate, trending_score, popularity_score,
                           user_rating, user_rating_count, critic_rating, critic_rating_count,
                           embedding, tfidf_vector
                    FROM item_features
                    WHERE genres @> $1::jsonb
                        AND is_active = true
                    ORDER BY popularity_score DESC NULLS LAST
                    LIMIT $2
                    "#,
                )
                .bind(&genre_json)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.by_genre");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch seasonal content by multiple tags with popularity and active status filter.
    pub async fn get_seasonal_content_by_tags(
        &self,
        search_tags: &[String],
        min_popularity: f32,
        limit: i64,
    ) -> Result<Vec<SeasonalItemRowExtended>> {
        let start = std::time::Instant::now();

        // Convert Vec<String> to JsonValue for PostgreSQL `?|` operator
        let search_tags_json = serde_json::to_value(search_tags)?;

        let rows: Vec<SeasonalItemRowExtended> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, SeasonalItemRowExtended>(
                    r#"
                    SELECT item_id, title, seasonal_tags, holiday_tags, themes, popularity_score
                    FROM item_features
                    WHERE is_active = true
                        AND (
                            seasonal_tags ?| $1
                            OR holiday_tags ?| $1
                            OR themes ?| $1
                        )
                        AND (popularity_score >= $2 OR popularity_score IS NULL)
                    ORDER BY popularity_score DESC NULLS LAST
                    LIMIT $3
                    "#,
                )
                .bind(&search_tags_json)
                .bind(min_popularity)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.seasonal_by_tags");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Get similar items based on embedding similarity for a given item.
    pub async fn get_item_similarities_by_embedding(
        &self,
        item_id: i32,
        limit: i64,
    ) -> Result<Vec<ItemSimilarityRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<ItemSimilarityRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemSimilarityRow>(
                    r#"
                    SELECT similar_item_id, similarity_score
                    FROM item_similarities
                    WHERE item_id = $1 AND similarity_type = 'embedding'
                    ORDER BY similarity_score DESC
                    LIMIT $2
                    "#,
                )
                .bind(item_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.item_similarities_embedding");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Get user's watchlist.
    pub async fn get_user_watchlist(
        &self,
        user_id: i32,
        limit: i64,
    ) -> Result<Vec<WatchlistItemRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<WatchlistItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, WatchlistItemRow>(
                    r#"
                    SELECT item_id, added_at
                    FROM user_watchlist
                    WHERE user_id = $1
                    ORDER BY added_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(user_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.watchlist");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Get subcategories for a given category slug.
    pub async fn get_subcategories_for_category(
        &self,
        category_slug: &str,
    ) -> Result<Vec<String>> {
        let start = std::time::Instant::now();

        #[derive(Debug, Clone, FromRow)]
        struct CategoryRow {
            slug: String,
        }

        let rows: Vec<CategoryRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, CategoryRow>(
                    r#"
                    SELECT slug FROM categories
                    WHERE parent_slug = $1 OR slug = $1
                    "#,
                )
                .bind(category_slug)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.subcategories");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| r.slug).collect())
    }

    /// Find items that users who watched source also watched.
    pub async fn get_co_watched_items(
        &self,
        item_id: i32,
        limit: i64,
    ) -> Result<Vec<(i32, i64)>> {
        let start = std::time::Instant::now();

        #[derive(sqlx::FromRow)]
        struct CoWatchedRow {
            item_id: i32,
            co_watch_count: i64,
        }

        let rows: Vec<CoWatchedRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, CoWatchedRow>(
                    r#"
                    SELECT ui2.item_id, COUNT(DISTINCT ui2.user_id) as co_watch_count
                    FROM user_interactions ui1
                    JOIN user_interactions ui2 ON ui1.user_id = ui2.user_id
                    WHERE ui1.item_id = $1
                        AND ui2.item_id != $1
                        AND ui1.interaction_type = 'view'
                        AND ui2.interaction_type = 'view'
                    GROUP BY ui2.item_id
                    ORDER BY co_watch_count DESC
                    LIMIT $2
                    "#,
                )
                .bind(item_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.co_watched");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| (r.item_id, r.co_watch_count)).collect())
    }

    /// Fetch items by categories with advanced filtering and sorting.
    pub async fn get_items_by_categories_advanced(
        &self,
        categories: &[String],
        min_rating: Option<f32>,
        sort_by: &str,
        limit: i64,
    ) -> Result<Vec<ItemFeatureRow>> {
        let start = std::time::Instant::now();

        let order_clause = match sort_by {
            "recent" => "release_date DESC NULLS LAST, added_date DESC NULLS LAST",
            "rating" => "COALESCE(user_rating, critic_rating, 0) DESC",
            "alphabetical" => "title ASC",
            _ => "popularity_score DESC NULLS LAST",
        };

        let mut query = format!(
            r#"
            SELECT i.item_id, i.title, i.description, i.genres, i.tags, i.creators,
                   i.directors, i.studios, i.actors,
                   i.content_type, i.language, i.audio_languages, i.subtitle_languages,
                   i.age_rating, i.duration_seconds, i.release_year, i.release_date,
                   i.published_at, i.added_date, i.is_active,
                   i.max_resolution, i.has_hdr, i.has_dolby_vision, i.has_dolby_atmos,
                   i.is_explicit, i.has_violence, i.has_strong_language, i.has_drug_content,
                   i.available_countries, i.blocked_countries,
                   i.seasonal_tags, i.holiday_tags, i.themes, i.is_award_winner,
                   i.view_count, i.like_count, i.comment_count, i.share_count, i.save_count,
                   i.completion_rate, i.trending_score, i.popularity_score,
                   i.user_rating, i.critic_rating,
                   i.embedding, i.tfidf_vector
            FROM item_features i
            JOIN item_categories ic ON i.item_id = ic.item_id
            WHERE ic.category_slug = ANY($1)
                AND i.is_active = true
            "#
        );

        if let Some(rating) = min_rating {
            query.push_str(&format!(
                " AND COALESCE(i.user_rating, i.critic_rating, 0) >= {}",
                rating
            ));
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        let cats = categories.to_vec();
        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(&query)
                    .bind(&cats)
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.by_categories");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch new releases with advanced filtering and sorting.
    pub async fn get_new_releases_advanced(
        &self,
        cutoff_date: DateTime<Utc>,
        content_type: &str,
        genre: Option<String>,
        sort_by: &str,
        limit: i64,
    ) -> Result<Vec<ItemFeatureRow>> {
        let start = std::time::Instant::now();

        let order_clause = match sort_by {
            "added_date" => "added_date DESC NULLS LAST",
            "popularity" => "popularity_score DESC NULLS LAST",
            _ => "release_date DESC NULLS LAST",
        };

        let mut query = format!(
            r#"
            SELECT item_id, title, description, genres, tags, creators,
                   directors, studios, actors,
                   content_type, language, audio_languages, subtitle_languages,
                   age_rating, duration_seconds, release_year, release_date,
                   published_at, added_date, is_active,
                   max_resolution, has_hdr, has_dolby_vision, has_dolby_atmos,
                   is_explicit, has_violence, has_strong_language, has_drug_content,
                   available_countries, blocked_countries,
                   seasonal_tags, holiday_tags, themes, is_award_winner,
                   view_count, like_count, comment_count, share_count, save_count,
                   completion_rate, trending_score, popularity_score,
                   user_rating, critic_rating,
                   embedding, tfidf_vector
            FROM item_features
            WHERE is_active = true
                AND (release_date >= $1 OR added_date >= $1)
            "#
        );

        if content_type != "all" {
            query.push_str(&format!(" AND content_type = '{}'", content_type));
        }

        if let Some(ref g) = genre {
            query.push_str(&format!(" AND genres @> '[\"{}\"]'::jsonb", g));
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(&query)
                    .bind(cutoff_date)
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.new_releases_advanced");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }


}

// ─── Additional row types ──────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct RecentWatchRow {
    pub item_id: i32,
    pub last_watched: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PopularItemRow {
    pub item_id: i32,
    pub view_count: i32,
    pub trending_score: f32,
    pub completion_rate: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct NewReleaseRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub trending_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct GenreItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub popularity_score: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SeasonalItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub seasonal_tags: Option<JsonValue>,
    pub trending_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct SeasonalItemRowExtended {
    pub item_id: i32,
    pub title: Option<String>,
    pub seasonal_tags: Option<JsonValue>,
    pub holiday_tags: Option<JsonValue>,
    pub themes: Option<JsonValue>,
    pub popularity_score: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ItemSimilarityRow {
    pub similar_item_id: i32,
    pub similarity_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct WatchlistItemRow {
    pub item_id: i32,
    pub added_at: DateTime<Utc>,
}


