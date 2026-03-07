use std::collections::HashMap;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use crate::db::repositories::item_feature_service::models::*;
use crate::db::repositories::item_feature_service::service::ItemFeatureService;

impl ItemFeatureService {
    /// Batch-fetch full item features for a set of item IDs.
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

    /// Get total count of active items in the catalog.
    pub async fn get_active_item_count(&self) -> Result<i64> {
        let start = std::time::Instant::now();

        let count: i64 = self
            .pool
            .execute(|pool| async move {
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM item_features WHERE is_active = true")
                    .fetch_one(&pool)
                    .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.count");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(count)
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
                    SELECT item_id, title, is_explicit, view_count, trending_score, 
                           completion_rate, age_rating, published_at, genres
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

    /// Fetch items by multiple genres.
    pub async fn get_items_by_genres(
        &self,
        genres: &[String],
        limit: i64,
    ) -> Result<Vec<GenreItemRow>> {
        let start = std::time::Instant::now();
        let genres_json = serde_json::to_value(genres)?;

        let rows: Vec<GenreItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, GenreItemRow>(
                    r#"
                    SELECT item_id, title, popularity_score
                    FROM item_features
                    WHERE genres ?| $1
                        AND is_active = true
                    ORDER BY popularity_score DESC NULLS LAST
                    LIMIT $2
                    "#,
                )
                .bind(&genres_json)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.by_genres");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
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

        let mut query = String::from(
            r#"
            SELECT i.item_id, i.title, i.description, i.genres, i.tags, i.creators,
                   i.directors, i.studios, i.actors,
                   i.content_type, i.language, i.audio_languages, i.subtitle_languages,
                   i.age_rating, i.duration_seconds, i.release_year, i.release_date,
                   i.published_at, i.added_date, i.available_from, i.available_until,
                   i.is_active,
                   i.max_resolution, i.has_hdr, i.has_dolby_vision, i.has_dolby_atmos,
                   i.is_explicit, i.has_violence, i.has_strong_language, i.has_drug_content,
                   i.available_countries, i.blocked_countries,
                   i.seasonal_tags, i.holiday_tags, i.themes,
                   i.is_award_winner, i.required_tier, i.is_free,
                   i.view_count, i.like_count, i.comment_count, i.share_count, i.save_count,
                   i.completion_rate, i.trending_score, i.popularity_score,
                   i.user_rating, i.user_rating_count, i.critic_rating, i.critic_rating_count,
                   i.embedding, i.tfidf_vector
            FROM item_features i
            JOIN item_categories ic ON i.item_id = ic.item_id
            WHERE ic.category_slug = ANY($1)
                AND i.is_active = true
            "#
        );

        if min_rating.is_some() {
            query.push_str(" AND COALESCE(i.user_rating, i.critic_rating, 0) >= $3");
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        let cats = categories.to_vec();
        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                let mut q = sqlx::query_as::<_, ItemFeatureRow>(&query)
                    .bind(&cats)
                    .bind(limit);
                
                if let Some(rating) = min_rating {
                    q = q.bind(rating);
                }
                
                q.fetch_all(&pool).await
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

        let mut query = String::from(
            r#"
            SELECT item_id, title, description, genres, tags, creators,
                   directors, studios, actors,
                   content_type, language, audio_languages, subtitle_languages,
                   age_rating, duration_seconds, release_year, release_date,
                   published_at, added_date, available_from, available_until,
                   is_active,
                   max_resolution, has_hdr, has_dolby_vision, has_dolby_atmos,
                   is_explicit, has_violence, has_strong_language, has_drug_content,
                   available_countries, blocked_countries,
                   seasonal_tags, holiday_tags, themes,
                   is_award_winner, required_tier, is_free,
                   view_count, like_count, comment_count, share_count, save_count,
                   completion_rate, trending_score, popularity_score,
                   user_rating, user_rating_count, critic_rating, critic_rating_count,
                   embedding, tfidf_vector
            FROM item_features
            WHERE is_active = true
                AND (release_date >= $1 OR added_date >= $1)
            "#
        );

        let mut next_bind = 3;
        if content_type != "all" {
            query.push_str(&format!(" AND content_type = ${}", next_bind));
            next_bind += 1;
        }

        if genre.is_some() {
            query.push_str(&format!(" AND genres @> ${}", next_bind));
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        let ct = content_type.to_string();
        let g_val = genre.map(|g| serde_json::json!([g]));

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                let mut q = sqlx::query_as::<_, ItemFeatureRow>(&query)
                    .bind(cutoff_date)
                    .bind(limit);
                
                if ct != "all" {
                    q = q.bind(ct);
                }
                
                if let Some(g) = g_val {
                    q = q.bind(g);
                }
                
                q.fetch_all(&pool).await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.new_releases_advanced");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

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
}
