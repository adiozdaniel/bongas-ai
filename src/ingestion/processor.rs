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
use tracing::{info, warn, error, debug};

use crate::db::ResilientPool;
use crate::db::repositories::interaction_repository::InteractionRepository;
use crate::engine::staleness_engine::{StalenessEngine, UserEvent};
use crate::resilience::ResilienceMetricsCollector;

use super::types::UserActivity;
use clickhouse::Row;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
pub struct ClickHouseInteraction {
    pub user_id: i32,
    pub item_id: i32,
    pub interaction_type: String,
    pub scenario_slug: String,
    pub rating: f32,
    pub watch_duration_seconds: i32,
    pub created_at: u64, // Unix timestamp for ClickHouse
}

/// Processes activities from any source and routes them to DB + staleness engine.
pub struct ActivityProcessor {
    interaction_repo: Arc<InteractionRepository>,
    pool: Arc<ResilientPool>,
    staleness_engine: Arc<StalenessEngine>,
    clickhouse: Option<Arc<clickhouse::Client>>,
}

impl ActivityProcessor {
    pub fn new(
        resilient_pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
        staleness_engine: Arc<StalenessEngine>,
        clickhouse: Option<Arc<clickhouse::Client>>,
    ) -> Self {
        Self {
            interaction_repo: Arc::new(InteractionRepository::new(resilient_pool.clone(), metrics_collector)),
            pool: resilient_pool,
            staleness_engine,
            clickhouse,
        }
    }

    /// Run the processor loop, consuming activities from the channel.
    pub async fn run(self: Arc<Self>, mut receiver: mpsc::Receiver<UserActivity>) {
        info!("Activity processor started (batched + async flush)");

        let mut buffer = Vec::with_capacity(100);
        let flush_interval = std::time::Duration::from_millis(500);
        let mut interval = tokio::time::interval(flush_interval);

        loop {
            tokio::select! {
                maybe_activity = receiver.recv() => {
                    match maybe_activity {
                        Some(activity) => {
                            buffer.push(activity);
                            if buffer.len() >= 100 {
                                let batch = std::mem::replace(&mut buffer, Vec::with_capacity(100));
                                let processor = self.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = processor.flush_batch(batch).await {
                                        error!("Failed to flush batch: {}", e);
                                    }
                                });
                            }
                        }
                        None => {
                            // Channel closed
                            if !buffer.is_empty() {
                                let batch = std::mem::take(&mut buffer);
                                let processor = self.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = processor.flush_batch(batch).await {
                                        error!("Failed to flush final batch: {}", e);
                                    }
                                });
                            }
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        let batch = std::mem::replace(&mut buffer, Vec::with_capacity(100));
                        let processor = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = processor.flush_batch(batch).await {
                                error!("Failed to flush batch on interval: {}", e);
                            }
                        });
                    }
                }
            }
        }

        warn!("Activity processor channel closed — shutting down");
    }

    /// Flush the buffered activities to DB and staleness engine
    async fn flush_batch(&self, buffer: Vec<UserActivity>) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let mut user_ids = Vec::with_capacity(buffer.len());
        let mut item_ids = Vec::with_capacity(buffer.len());
        let mut types = Vec::with_capacity(buffer.len());
        let mut ratings = Vec::with_capacity(buffer.len());
        let mut durations = Vec::with_capacity(buffer.len());

        let mut processed_indices = Vec::new();
        let mut clickhouse_rows = Vec::with_capacity(buffer.len());
        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for (i, activity) in buffer.iter().enumerate() {
            match activity {
                UserActivity::Playback { user_id, item_id, watch_percentage, watch_duration_seconds, completed, scenario_slug, .. } => {
                    // Logic from process_playback
                    let mut rating = match watch_percentage {
                        p if *p < 0.25 => 1.0,
                        p if *p < 0.50 => 2.0,
                        p if *p < 0.75 => 3.5,
                        _ => 5.0,
                    };
                    if *completed {
                        rating = (rating + 0.5_f32).min(5.0_f32);
                    }

                    user_ids.push(*user_id);
                    item_ids.push(*item_id);
                    types.push("implicit_rating".to_string());
                    ratings.push(Some(rating));
                    durations.push(Some(*watch_duration_seconds));
                    processed_indices.push(i);

                    clickhouse_rows.push(ClickHouseInteraction {
                        user_id: *user_id,
                        item_id: *item_id,
                        interaction_type: "playback".to_string(),
                        scenario_slug: scenario_slug.clone().unwrap_or_else(|| "unknown".to_string()),
                        rating,
                        watch_duration_seconds: *watch_duration_seconds,
                        created_at: now_ts,
                    });
                }
                UserActivity::Reaction { user_id, item_id, reaction_type, scenario_slug, .. } => {
                    // Logic from process_reaction
                    let rating = match reaction_type.as_str() {
                        "like" => 5.0,
                        "dislike" => 1.0,
                        _ => 3.0,
                    };

                    user_ids.push(*user_id);
                    item_ids.push(*item_id);
                    types.push("explicit_rating".to_string());
                    ratings.push(Some(rating));
                    durations.push(None);
                    processed_indices.push(i);

                    clickhouse_rows.push(ClickHouseInteraction {
                        user_id: *user_id,
                        item_id: *item_id,
                        interaction_type: reaction_type.clone(),
                        scenario_slug: scenario_slug.clone().unwrap_or_else(|| "unknown".to_string()),
                        rating,
                        watch_duration_seconds: 0,
                        created_at: now_ts,
                    });
                }
                UserActivity::Click { user_id, item_id, scenario_slug, .. } => {
                    user_ids.push(*user_id);
                    item_ids.push(*item_id);
                    types.push("click".to_string());
                    ratings.push(None);
                    durations.push(None);
                    processed_indices.push(i);

                    clickhouse_rows.push(ClickHouseInteraction {
                        user_id: *user_id,
                        item_id: *item_id,
                        interaction_type: "click".to_string(),
                        scenario_slug: scenario_slug.clone().unwrap_or_else(|| "unknown".to_string()),
                        rating: 0.0,
                        watch_duration_seconds: 0,
                        created_at: now_ts,
                    });
                }
                UserActivity::Impression { user_id, item_id, scenario_slug, .. } => {
                    user_ids.push(*user_id);
                    item_ids.push(*item_id);
                    types.push("impression".to_string());
                    ratings.push(None);
                    durations.push(None);
                    processed_indices.push(i);

                    clickhouse_rows.push(ClickHouseInteraction {
                        user_id: *user_id,
                        item_id: *item_id,
                        interaction_type: "impression".to_string(),
                        scenario_slug: scenario_slug.clone().unwrap_or_else(|| "unknown".to_string()),
                        rating: 0.0,
                        watch_duration_seconds: 0,
                        created_at: now_ts,
                    });
                }
                // Handle complex types individually
                UserActivity::ProfileUpdate { user_id, update_type, data, .. } => {
                    if let Err(e) = self.process_profile_update(*user_id, update_type, data).await {
                        error!(user_id, error = %e, "Failed to process profile update in batch");
                    }
                }
                UserActivity::Notification { user_id, notification_type, .. } => {
                    if let Err(e) = self.process_notification(*user_id, notification_type).await {
                        error!(user_id, error = %e, "Failed to process notification in batch");
                    }
                }
            }
        }

        // Batch insert interactions into Postgres
        if !user_ids.is_empty() {
            if let Err(e) = self.interaction_repo.create_interactions_batch(
                user_ids.clone(), item_ids, types, ratings, durations
            ).await {
                error!("Failed to batch insert interactions: {}", e);
            }

            // Update arrival patterns for all unique users in this batch
            let unique_users: std::collections::HashSet<i32> = user_ids.into_iter().collect();
            for uid in unique_users {
                let _ = self.interaction_repo.update_arrival_pattern(uid).await;
            }
        }

        // Batch insert into ClickHouse (The OLAP Path)
        if !clickhouse_rows.is_empty() {
            if let Some(ref ch) = self.clickhouse {
                let mut inserter = ch.insert::<ClickHouseInteraction>("user_interactions").await
                    .map_err(|e| anyhow::anyhow!("ClickHouse insert preparation failed: {}", e))?;

                for row in clickhouse_rows {
                    inserter.write(&row).await
                        .map_err(|e| anyhow::anyhow!("ClickHouse write failed: {}", e))?;
                }

                inserter.end().await
                    .map_err(|e| anyhow::anyhow!("ClickHouse commit failed: {}", e))?;

                debug!(count = buffer.len(), "Batch inserted into ClickHouse");
            }
        }

        // Update staleness engine for all events
        for activity in buffer.iter() {
            let event = match activity {
                UserActivity::Playback { user_id, item_id, watch_percentage, watch_duration_seconds, .. } => {
                    if *watch_duration_seconds < 5 {
                        Some(UserEvent::NegativeSignal {
                            user_id: *user_id,
                            item_id: *item_id,
                        })
                    } else {
                        Some(UserEvent::WatchEvent {
                            user_id: *user_id,
                            item_id: *item_id,
                            completion_rate: *watch_percentage,
                        })
                    }
                },
                UserActivity::Reaction { user_id, item_id, reaction_type, .. } => {
                    let rating = match reaction_type.as_str() {
                        "like" => 5.0,
                        "dislike" => 1.0,
                        _ => 3.0,
                    };
                    Some(UserEvent::ExplicitFeedback {
                        user_id: *user_id,
                        item_id: *item_id,
                        rating,
                    })
                }
                UserActivity::ProfileUpdate { user_id, .. } => Some(UserEvent::ExplicitFeedback {
                    user_id: *user_id,
                    item_id: 0,
                    rating: 0.0,
                }),
                UserActivity::Notification { notification_type, .. } if notification_type == "new_content" => {
                     Some(UserEvent::NewContentInGenre { genre: "all".to_string() })
                },
                 UserActivity::Notification { user_id, notification_type, .. } if notification_type == "recommendation" => {
                     Some(UserEvent::ExplicitFeedback {
                        user_id: *user_id,
                        item_id: 0,
                        rating: 0.0,
                    })
                },
                _ => None,
            };

            if let Some(ev) = event {
                if let Err(e) = self.staleness_engine.process_event(&ev).await {
                     warn!(error = %e, "Failed to process staleness event");
                }
            }
        }

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
