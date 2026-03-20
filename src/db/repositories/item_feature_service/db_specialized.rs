use anyhow::Result;
use crate::db::repositories::item_feature_service::models::*;
use crate::db::repositories::item_feature_service::service::ItemFeatureService;
use crate::error::{AppError, PostgresError};

impl ItemFeatureService {
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
                    FROM bongas.user_item_scores
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

    /// Get candidate item IDs for similarity search.
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
                    SELECT item_id FROM bongas.item_features
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
                    FROM bongas.item_features
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

    /// Get seasonal content by multiple tags.
    pub async fn get_seasonal_content_by_tags(
        &self,
        search_tags: &[String],
        min_popularity: f32,
        limit: i64,
    ) -> Result<Vec<SeasonalItemRowExtended>> {
        let start = std::time::Instant::now();
        let search_tags_json = serde_json::to_value(search_tags)?;

        let rows: Vec<SeasonalItemRowExtended> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, SeasonalItemRowExtended>(
                    r#"
                    SELECT item_id, title, seasonal_tags, holiday_tags, themes, popularity_score
                    FROM bongas.item_features
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

    /// Get similar items based on embedding similarity.
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
                    FROM bongas.item_similarities
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

    /// Get subcategories for a given category slug.
    pub async fn get_subcategories_for_category(
        &self,
        category_slug: &str,
    ) -> Result<Vec<String>> {
        let start = std::time::Instant::now();

        #[derive(Debug, Clone, sqlx::FromRow)]
        struct CategoryRow { slug: String }

        let rows: Vec<CategoryRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, CategoryRow>(
                    r#"
                    SELECT slug FROM bongas.categories
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
                    WITH target_users AS (
                        SELECT DISTINCT user_id 
                        FROM bongas.user_interactions 
                        WHERE item_id = $1 
                          AND interaction_type = 'view'
                          AND created_at > (now() - INTERVAL '30 days')
                    )
                    SELECT ui.item_id, COUNT(*) as co_watch_count
                    FROM bongas.user_interactions ui

                    JOIN target_users tu ON ui.user_id = tu.user_id
                    WHERE ui.item_id != $1
                      AND ui.interaction_type = 'view'
                      AND ui.created_at > (now() - INTERVAL '30 days')
                    GROUP BY ui.item_id
                    ORDER BY co_watch_count DESC
                    LIMIT $2
                    "#,
                )
                .bind(item_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| AppError::Postgres(PostgresError::Query {
                message: format!("Failed to fetch co-watched items: {}", e),
                source: None,
            }))?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.co_watched");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| (r.item_id, r.co_watch_count)).collect())
    }

    /// Get recent interaction item IDs for a user.
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
                    FROM bongas.user_interactions
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
}
