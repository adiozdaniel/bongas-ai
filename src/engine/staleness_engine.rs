use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug};

use crate::engine::staging_manager::StagingManager;

pub struct StalenessEngine {
    staging_manager: Arc<StagingManager>,
}

impl StalenessEngine {
    pub fn new(staging_manager: Arc<StagingManager>) -> Self {
        Self { staging_manager }
    }

    /// Invalidate caches based on user event
    pub async fn on_user_event(&self, event: &UserEvent) -> Result<()> {
        match event {
            UserEvent::NewWatch { user_id, item_id } => {
                self.on_new_watch(*user_id, *item_id).await?;
            }
            UserEvent::ExplicitFeedback { user_id, item_id, rating } => {
                self.on_explicit_feedback(*user_id, *item_id, *rating).await?;
            }
            UserEvent::CompleteWatch { user_id, item_id } => {
                self.on_complete_watch(*user_id, *item_id).await?;
            }
            UserEvent::NewContentInGenre { genre } => {
                self.on_new_content_in_genre(genre).await?;
            }
        }

        Ok(())
    }

    /// Invalidate continue_watching scenario after new watch
    async fn on_new_watch(&self, user_id: i32, item_id: i32) -> Result<()> {
        debug!(user_id = user_id, item_id = item_id, "New watch event");

        self.staging_manager.mark_stale(
            user_id,
            Some("continue_watching"),
            "new_watch_event",
        ).await?;

        Ok(())
    }

    /// Invalidate personalized scenarios after explicit feedback
    async fn on_explicit_feedback(&self, user_id: i32, item_id: i32, rating: f32) -> Result<()> {
        info!(user_id = user_id, item_id = item_id, rating = rating, "Explicit feedback event");

        let scenarios = vec![
            "for_you_personalized",
            "because_you_watched",
        ];

        for scenario in scenarios {
            self.staging_manager.mark_stale(
                user_id,
                Some(scenario),
                "explicit_feedback",
            ).await?;
        }

        Ok(())
    }

    /// Invalidate completion-based scenarios
    async fn on_complete_watch(&self, user_id: i32, item_id: i32) -> Result<()> {
        debug!(user_id = user_id, item_id = item_id, "Complete watch event");

        self.staging_manager.mark_stale(
            user_id,
            Some("continue_watching"),
            "completed_watch",
        ).await?;

        Ok(())
    }

    /// Handle new content in genre (background worker handles broader invalidation)
    async fn on_new_content_in_genre(&self, genre: &str) -> Result<()> {
        info!(genre = genre, "New content in genre");
        // Genre-based invalidation is handled by background workers
        // or by marking trending scenarios as stale globally
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum UserEvent {
    NewWatch { user_id: i32, item_id: i32 },
    ExplicitFeedback { user_id: i32, item_id: i32, rating: f32 },
    CompleteWatch { user_id: i32, item_id: i32 },
    NewContentInGenre { genre: String },
}
