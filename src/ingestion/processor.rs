//! Activity processor — single consumer of the ingestion channel.
//!
//! Normalizes `UserActivity` from any source into:
//! 1. Database writes (via `InteractionRepository` / direct SQL for profiles)
//! 2. `UserEvent`s fed to the `StalenessEngine` for cache invalidation
//!
//! All DB access goes through existing resilient infrastructure
//! (circuit breakers, bulkhead, metrics).

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

use crate::db::ResilientPool;
use crate::db::repositories::interaction_repository::InteractionRepository;
use crate::engine::staleness_engine::{StalenessEngine, UserEvent};
use crate::resilience::ResilienceMetricsCollector;

use super::types::UserActivity;

/// Processes activities from any source and routes them to DB + staleness engine.
pub struct ActivityProcessor {
    interaction_repo: Arc<InteractionRepository>,
    pool: Arc<ResilientPool>,
    staleness_engine: Arc<StalenessEngine>,
}

impl ActivityProcessor {
    pub fn new(
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
    ) -> Self {
        Self {
            interaction_repo: Arc::new(InteractionRepository::new(resilient_pool.clone(), metrics_collector)),
            pool: resilient_pool,
            staleness_engine,
        }
    }

    /// Run the processor loop, consuming activities from the channel.
    pub async fn run(self: Arc<Self>, mut receiver: mpsc::Receiver<UserActivity>) {
        info!("Activity processor started");

        while let Some(activity) = receiver.recv().await {
            let kind = activity.kind();
            let user_id = activity.user_id();

            if let Err(e) = self.process(activity).await {
                error!(
                    user_id,
                    activity_type = kind,
                    error = %e,
                    "Failed to process activity"
                );
            }
        }

        warn!("Activity processor channel closed — shutting down");
    }

    /// Process a single activity: DB write + staleness event.
    async fn process(&self, activity: UserActivity) -> Result<()> {
        match activity {
            UserActivity::Playback {
                user_id,
                item_id,
                watch_percentage,
                watch_duration_seconds,
                completed,
                ..
            } => {
                self.process_playback(user_id, item_id, watch_percentage, watch_duration_seconds, completed).await
            }

            UserActivity::Reaction {
                user_id,
                item_id,
                reaction_type,
                ..
            } => {
                self.process_reaction(user_id, item_id, &reaction_type).await
            }

            UserActivity::ProfileUpdate {
                user_id,
                update_type,
                data,
                ..
            } => {
                self.process_profile_update(user_id, &update_type, &data).await
            }

            UserActivity::Notification {
                user_id,
                notification_type,
                ..
            } => {
                self.process_notification(user_id, &notification_type).await
            }

            UserActivity::Click {
                user_id,
                item_id,
                ..
            } => {
                self.interaction_repo.record_click(user_id, item_id).await?;
                info!(user_id, item_id, "Processed click");
                Ok(())
            }

            UserActivity::Impression {
                user_id,
                item_id,
                ..
            } => {
                self.interaction_repo.record_impression(user_id, item_id).await?;
                Ok(())
            }
        }
    }

    // ─── Playback Processing ────────────────────────────────────────────────

    async fn process_playback(
        &self,
        user_id: i32,
        item_id: i32,
        watch_percentage: f32,
        watch_duration_seconds: i32,
        completed: bool,
    ) -> Result<()> {
        // Convert watch percentage to implicit rating (1-5 scale)
        let mut rating = match watch_percentage {
            p if p < 0.25 => 1.0,
            p if p < 0.50 => 2.0,
            p if p < 0.75 => 3.5,
            _ => 5.0,
        };

        // Bonus for completion
        if completed {
            rating = (rating + 0.5_f32).min(5.0_f32);
        }

        // Store via resilient repository
        self.interaction_repo
            .create_implicit_rating(user_id, item_id, rating, watch_duration_seconds)
            .await?;

        // Invalidate caches
        self.staleness_engine
            .process_event(&UserEvent::WatchEvent {
                user_id,
                item_id,
                completion_rate: watch_percentage,
            })
            .await?;

        info!(user_id, item_id, watch_percentage, rating, "Processed playback");
        Ok(())
    }

    // ─── Reaction Processing ────────────────────────────────────────────────

    async fn process_reaction(
        &self,
        user_id: i32,
        item_id: i32,
        reaction_type: &str,
    ) -> Result<()> {
        let rating = match reaction_type {
            "like" => 5.0,
            "dislike" => 1.0,
            _ => 3.0,
        };

        self.interaction_repo
            .create_explicit_rating(user_id, item_id, rating)
            .await?;

        self.staleness_engine
            .process_event(&UserEvent::ExplicitFeedback {
                user_id,
                item_id,
                rating,
            })
            .await?;

        info!(user_id, item_id, reaction_type, "Processed reaction");
        Ok(())
    }

    // ─── Profile Update Processing ──────────────────────────────────────────

    async fn process_profile_update(
        &self,
        user_id: i32,
        update_type: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        match update_type {
            "preferences" => self.update_preferences(user_id, data).await?,
            "settings" => self.update_settings(user_id, data).await?,
            "demographics" => self.update_demographics(user_id, data).await?,
            "subscription" => self.update_subscription(user_id, data).await?,
            other => {
                warn!(user_id, update_type = other, "Unknown profile update type");
                return Ok(());
            }
        }

        // All profile changes invalidate personalized recommendations
        self.staleness_engine
            .process_event(&UserEvent::ExplicitFeedback {
                user_id,
                item_id: 0,
                rating: 0.0,
            })
            .await?;

        info!(user_id, update_type, "Processed profile update");
        Ok(())
    }

    async fn update_preferences(&self, user_id: i32, data: &serde_json::Value) -> Result<()> {
        let preferred_genres: Option<serde_json::Value> = data.get("preferred_genres")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .map(|g| serde_json::json!(g));

        let preferred_languages: Option<serde_json::Value> = data.get("preferred_languages")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .map(|l| serde_json::json!(l));

        let content_maturity: Option<String> = data.get("content_maturity")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        self.pool.execute(|pool| {
            let preferred_genres = preferred_genres.clone();
            let preferred_languages = preferred_languages.clone();
            let content_maturity = content_maturity.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_preferences (user_id, preferred_genres, preferred_languages, content_maturity, updated_at)
                    VALUES ($1, $2, $3, $4, NOW())
                    ON CONFLICT (user_id) DO UPDATE SET
                        preferred_genres = COALESCE($2, user_preferences.preferred_genres),
                        preferred_languages = COALESCE($3, user_preferences.preferred_languages),
                        content_maturity = COALESCE($4, user_preferences.content_maturity),
                        updated_at = NOW()
                    "#
                )
                .bind(user_id)
                .bind(preferred_genres)
                .bind(preferred_languages)
                .bind(content_maturity)
                .execute(&pool)
                .await
            }
        })
        .await?;

        Ok(())
    }

    async fn update_settings(&self, user_id: i32, data: &serde_json::Value) -> Result<()> {
        let notifications_enabled: Option<bool> = data.get("notifications_enabled")
            .and_then(|v| v.as_bool());
        let autoplay_enabled: Option<bool> = data.get("autoplay_enabled")
            .and_then(|v| v.as_bool());
        let video_quality: Option<String> = data.get("video_quality")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        self.pool.execute(|pool| {
            let video_quality = video_quality.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_settings (user_id, notifications_enabled, autoplay_enabled, video_quality, updated_at)
                    VALUES ($1, $2, $3, $4, NOW())
                    ON CONFLICT (user_id) DO UPDATE SET
                        notifications_enabled = COALESCE($2, user_settings.notifications_enabled),
                        autoplay_enabled = COALESCE($3, user_settings.autoplay_enabled),
                        video_quality = COALESCE($4, user_settings.video_quality),
                        updated_at = NOW()
                    "#
                )
                .bind(user_id)
                .bind(notifications_enabled)
                .bind(autoplay_enabled)
                .bind(video_quality)
                .execute(&pool)
                .await
            }
        })
        .await?;

        Ok(())
    }

    async fn update_demographics(&self, user_id: i32, data: &serde_json::Value) -> Result<()> {
        let age_group: Option<String> = data.get("age_group")
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let country: Option<String> = data.get("country")
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let timezone: Option<String> = data.get("timezone")
            .and_then(|v| v.as_str()).map(|s| s.to_string());

        self.pool.execute(|pool| {
            let age_group = age_group.clone();
            let country = country.clone();
            let timezone = timezone.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_demographics (user_id, age_group, country, timezone, updated_at)
                    VALUES ($1, $2, $3, $4, NOW())
                    ON CONFLICT (user_id) DO UPDATE SET
                        age_group = COALESCE($2, user_demographics.age_group),
                        country = COALESCE($3, user_demographics.country),
                        timezone = COALESCE($4, user_demographics.timezone),
                        updated_at = NOW()
                    "#
                )
                .bind(user_id)
                .bind(age_group)
                .bind(country)
                .bind(timezone)
                .execute(&pool)
                .await
            }
        })
        .await?;

        Ok(())
    }

    async fn update_subscription(&self, user_id: i32, data: &serde_json::Value) -> Result<()> {
        let tier: Option<String> = data.get("tier")
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let is_active: Option<bool> = data.get("is_active")
            .and_then(|v| v.as_bool());
        let expires_at: Option<chrono::DateTime<chrono::Utc>> = data.get("expires_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        self.pool.execute(|pool| {
            let tier = tier.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_subscriptions (user_id, tier, is_active, expires_at, updated_at)
                    VALUES ($1, $2, $3, $4, NOW())
                    ON CONFLICT (user_id) DO UPDATE SET
                        tier = COALESCE($2, user_subscriptions.tier),
                        is_active = COALESCE($3, user_subscriptions.is_active),
                        expires_at = COALESCE($4, user_subscriptions.expires_at),
                        updated_at = NOW()
                    "#
                )
                .bind(user_id)
                .bind(tier)
                .bind(is_active)
                .bind(expires_at)
                .execute(&pool)
                .await
            }
        })
        .await?;

        Ok(())
    }

    // ─── Notification Processing ────────────────────────────────────────────

    async fn process_notification(
        &self,
        user_id: i32,
        notification_type: &str,
    ) -> Result<()> {
        // Invalidate caches based on notification type
        match notification_type {
            "new_content" => {
                self.staleness_engine
                    .process_event(&UserEvent::NewContentInGenre {
                        genre: "all".to_string(),
                    })
                    .await?;
            }
            "recommendation" => {
                self.staleness_engine
                    .process_event(&UserEvent::ExplicitFeedback {
                        user_id,
                        item_id: 0,
                        rating: 0.0,
                    })
                    .await?;
            }
            _ => {}
        }

        info!(user_id, notification_type, "Processed notification");
        Ok(())
    }
}
