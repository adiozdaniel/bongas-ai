use anyhow::Result;
use crate::db::repositories::item_feature_service::models::*;
use crate::db::repositories::item_feature_service::service::ItemFeatureService;

impl ItemFeatureService {
    /// Get profile features for a single profile.
    pub async fn get_profile_features(&self, profile_id: &str) -> Result<Option<ProfileFeatureRow>> {
        let start = std::time::Instant::now();
        let pid = profile_id.to_string();

        let row: Option<ProfileFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ProfileFeatureRow>(
                    r#"
                    SELECT profile_id, user_id, genre_affinity, disliked_genres, total_watch_time_minutes,
                           total_videos_watched, avg_completion_rate,
                           favorite_genres, favorite_creators, preferred_content_type, embedding
                    FROM profile_features
                    WHERE profile_id = $1
                    "#,
                )
                .bind(pid)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.profile");
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
}
