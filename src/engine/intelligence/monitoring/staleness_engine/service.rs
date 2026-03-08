use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, debug};

use crate::engine::execution::cache::staging_manager::service::StagingManager;
use crate::db::ItemFeatureService;

pub struct StalenessEngine {
    rules: Vec<StalenessRule>,
    staging_manager: Arc<StagingManager>,
    item_feature_service: Arc<ItemFeatureService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserEvent {
    /// User watched content (playback session)
    WatchEvent {
        user_id: i32,
        profile_id: String,
        item_id: i32,
        completion_rate: f32,
    },

    /// User liked/disliked content
    ExplicitFeedback {
        user_id: i32,
        profile_id: String,
        item_id: i32,
        rating: f32,
    },

    /// User completed watching content (>90% completion)
    CompleteWatch {
        user_id: i32,
        profile_id: String,
        item_id: i32,
    },

    /// New content added to platform
    NewContentInGenre {
        genre: String,
    },

    /// User explicitly skipped content (negative signal)
    NegativeSignal {
        user_id: i32,
        profile_id: String,
        item_id: i32,
    },

    /// Hourly tick for time-based invalidation
    HourlyTick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StalenessRule {
    /// Invalidate continue_watching if new watch event
    OnNewWatchEvent {
        scenarios: Vec<String>,
    },

    /// Invalidate scenario hourly (trending, new releases)
    TimeBasedInvalidation {
        scenarios: Vec<String>,
        ttl_hours: u64,
    },

    /// Invalidate if user liked/disliked
    OnExplicitFeedback {
        scenarios: Vec<String>,
    },

    /// Invalidate if new content added to genre user watches
    OnNewContentInGenre {
        scenarios: Vec<String>,
    },
}

impl StalenessEngine {
    pub fn new(
        staging_manager: Arc<StagingManager>,
        item_feature_service: Arc<ItemFeatureService>,
    ) -> Self {
        let rules = vec![
            // Continue watching invalidates on new watch
            StalenessRule::OnNewWatchEvent {
                scenarios: vec![
                    "continue_watching".to_string(),
                    "personalized_continue_watching".to_string(),
                ],
            },

            // Trending invalidates hourly
            StalenessRule::TimeBasedInvalidation {
                scenarios: vec![
                    "trending_now".to_string(),
                    "new_releases".to_string(),
                ],
                ttl_hours: 1,
            },

            // Personalized scenarios invalidate on feedback
            StalenessRule::OnExplicitFeedback {
                scenarios: vec![
                    "for_you_personalized".to_string(),
                    "because_you_watched".to_string(),
                    "recommended_for_you".to_string(),
                    "supreme_ranker".to_string(),
                ],
            },

            // Genre-based scenarios invalidate on new content
            StalenessRule::OnNewContentInGenre {
                scenarios: vec![
                    "genre_picks".to_string(),
                ],
            },
        ];

        Self {
            rules,
            staging_manager,
            item_feature_service,
        }
    }

    /// Process user event and invalidate caches if needed
    pub async fn process_event(&self, event: &UserEvent) -> Result<()> {
        match event {
            UserEvent::WatchEvent { user_id, profile_id, item_id, completion_rate } => {
                self.handle_watch_event(*user_id, profile_id, *item_id, *completion_rate).await?;
            }
            UserEvent::ExplicitFeedback { user_id, profile_id, item_id, rating } => {
                self.handle_explicit_feedback(*user_id, profile_id, *item_id, *rating).await?;
            }
            UserEvent::CompleteWatch { user_id, profile_id, item_id } => {
                self.handle_complete_watch(*user_id, profile_id, *item_id).await?;
            }
            UserEvent::NewContentInGenre { genre } => {
                self.handle_new_content(genre).await?;
            }
            UserEvent::NegativeSignal { user_id, profile_id, item_id } => {
                self.handle_negative_signal(*user_id, profile_id, *item_id).await?;
            }
            UserEvent::HourlyTick => {
                self.handle_hourly_tick().await?;
            }
        }

        Ok(())
    }

    /// Alias for backward compatibility with BongasEngine
    pub async fn on_user_event(&self, event: &UserEvent) -> Result<()> {
        self.process_event(event).await
    }

    async fn handle_watch_event(
        &self,
        user_id: i32,
        profile_id: &str,
        item_id: i32,
        completion_rate: f32,
    ) -> Result<()> {
        debug!(
            user_id = user_id,
            profile_id = %profile_id,
            item_id = item_id,
            completion_rate = completion_rate,
            "Processing watch event"
        );

        for rule in &self.rules {
            if let StalenessRule::OnNewWatchEvent { scenarios } = rule {
                for scenario_slug in scenarios {
                    self.staging_manager.invalidate(scenario_slug, user_id, Some(profile_id)).await?;
                    info!(
                        user_id = user_id,
                        profile_id = %profile_id,
                        item_id = item_id,
                        scenario_slug = %scenario_slug,
                        completion_rate = completion_rate,
                        "Invalidated cache due to watch event"
                    );
                }
            }
        }

        // If completion is high (>90%), also treat as complete watch
        if completion_rate > 0.9 {
            self.handle_complete_watch(user_id, profile_id, item_id).await?;
        }

        Ok(())
    }

    async fn handle_explicit_feedback(
        &self,
        user_id: i32,
        profile_id: &str,
        item_id: i32,
        rating: f32,
    ) -> Result<()> {
        info!(
            user_id = user_id,
            profile_id = %profile_id,
            item_id = item_id,
            rating = rating,
            "Processing explicit feedback"
        );

        for rule in &self.rules {
            if let StalenessRule::OnExplicitFeedback { scenarios } = rule {
                for scenario_slug in scenarios {
                    self.staging_manager.invalidate(scenario_slug, user_id, Some(profile_id)).await?;
                    info!(
                        user_id = user_id,
                        profile_id = %profile_id,
                        item_id = item_id,
                        rating = rating,
                        scenario_slug = %scenario_slug,
                        "Invalidated cache due to explicit feedback"
                    );
                }
            }
        }

        Ok(())
    }

    async fn handle_complete_watch(&self, user_id: i32, profile_id: &str, item_id: i32) -> Result<()> {
        debug!(user_id = user_id, profile_id = %profile_id, item_id = item_id, "Complete watch event");

        self.staging_manager.mark_stale(
            user_id,
            Some(profile_id),
            Some("continue_watching"),
            "completed_watch",
        ).await?;

        Ok(())
    }

    async fn handle_new_content(&self, genre: &str) -> Result<()> {
        info!(genre = genre, "Processing new content event, triggering genre-based invalidation");

        for rule in &self.rules {
            if let StalenessRule::OnNewContentInGenre { scenarios } = rule {
                for scenario_slug in scenarios {
                    // Global invalidation for this scenario across all users
                    // In a production system, we'd use a pattern-based L2 invalidation
                    // and let L1 expire or use a global broadcast.
                    let pattern = format!("rec:{}:*", scenario_slug);
                    let _ = self.staging_manager.cache_manager().delete_pattern(&pattern).await;
                    
                    info!(
                        scenario_slug = %scenario_slug,
                        genre = genre,
                        "Invalidated global cache for scenario due to new content"
                    );
                }
            }
        }

        Ok(())
    }

    async fn handle_negative_signal(
        &self,
        user_id: i32,
        profile_id: &str,
        item_id: i32,
    ) -> Result<()> {
        info!(user_id, profile_id = %profile_id, item_id, "Processing negative signal (Skip)");
        
        // 1. Invalidate caches immediately
        self.staging_manager.invalidate("supreme_ranker", user_id, Some(profile_id)).await?;
        self.staging_manager.invalidate("for_you_personalized", user_id, Some(profile_id)).await?;

        // 2. Fetch actual genres for the skipped item
        let genres = match self.item_feature_service.get_item_features_batch(&[item_id]).await {
            Ok(features) => {
                features.get(&item_id)
                    .and_then(|f| f.genres.as_ref())
                    .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
                    .unwrap_or_else(|| vec!["unknown".to_string()])
            }
            Err(e) => {
                warn!(error = %e, item_id, "Failed to fetch item genres for skip burner, falling back to unknown");
                vec!["unknown".to_string()]
            }
        };

        // 3. Record penalty (Genre Burn)
        self.staging_manager.record_negative_signal(user_id, Some(profile_id), genres).await?;

        Ok(())
    }

    async fn handle_hourly_tick(&self) -> Result<()> {
        for rule in &self.rules {
            if let StalenessRule::TimeBasedInvalidation { scenarios, ttl_hours } = rule {
                info!(
                    scenarios = ?scenarios,
                    ttl_hours = ttl_hours,
                    "Hourly tick - time-based invalidation rules apply"
                );
                // PostgreSQL TTL handles automatic expiration
                // This logs for monitoring purposes
            }
        }

        // Cleanup expired L2 entries
        if let Err(e) = self.staging_manager.cleanup_expired().await {
            warn!(error = %e, "Failed to cleanup expired cache entries");
        }

        Ok(())
    }

    /// Check if cache should be invalidated for a given event
    pub fn should_invalidate(
        &self,
        scenario_slug: &str,
        event: &UserEvent,
    ) -> bool {
        for rule in &self.rules {
            match (rule, event) {
                (StalenessRule::OnNewWatchEvent { scenarios }, UserEvent::WatchEvent { .. }) => {
                    if scenarios.iter().any(|s| s == scenario_slug) {
                        return true;
                    }
                }
                (StalenessRule::OnExplicitFeedback { scenarios }, UserEvent::ExplicitFeedback { .. }) => {
                    if scenarios.iter().any(|s| s == scenario_slug) {
                        return true;
                    }
                }
                (StalenessRule::OnNewContentInGenre { scenarios }, UserEvent::NewContentInGenre { .. }) => {
                    if scenarios.iter().any(|s| s == scenario_slug) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

}
