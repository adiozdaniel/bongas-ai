use anyhow::Result;
use crate::db::repositories::item_feature_service::models::*;
use crate::db::repositories::item_feature_service::service::ItemFeatureService;

impl ItemFeatureService {
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
                    FROM bongas.user_interactions
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

    /// Get user's most recently watched item with a minimum completion rate.
    pub async fn get_recent_watch_with_completion(
        &self,
        user_id: i32,
        min_completion: f32,
    ) -> Result<Option<i32>> {
        let start = std::time::Instant::now();

        #[derive(sqlx::FromRow)]
        struct Row { item_id: i32 }

        let row: Option<Row> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, Row>(
                    r#"
                    SELECT item_id
                    FROM bongas.user_interactions
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
                    FROM bongas.user_watchlist
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
}
