use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::analytics::Analytics;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::models::UserFeatures;

pub struct UserFeatureComputer {
    analytics: Arc<Analytics>,
    feature_repo: Arc<FeatureRepository>,
}

impl UserFeatureComputer {
    pub fn new(analytics: Arc<Analytics>, feature_repo: Arc<FeatureRepository>) -> Self {
        Self { analytics, feature_repo }
    }

    /// Compute all features for a user
    pub async fn compute(&self, user_id: i32) -> Result<UserFeatures> {
        // Get user behavior stats from ClickHouse
        let behavior = self.analytics.user_behavior.get_user_stats(user_id as u32).await?;

        // Compute genre affinity (TF-IDF style)
        let genre_affinity = self.compute_genre_affinity(user_id).await?;

        // Compute watch patterns
        let watch_patterns = json!({
            "favorite_time_of_day": behavior.favorite_time_of_day,
            "active_days_last_30d": behavior.active_days_last_30d,
            "completion_histogram": behavior.completion_histogram,
        });

        let features = UserFeatures {
            user_id,
            genre_affinity: Some(json!(genre_affinity)),
            total_watch_time_minutes: behavior.total_watch_time_minutes as i32,
            total_videos_watched: behavior.total_videos_watched as i32,
            avg_completion_rate: behavior.avg_completion_rate,
            favorite_genres: Some(self.compute_favorite_genres(&genre_affinity)),
            watch_patterns: Some(watch_patterns),
            features_updated_at: chrono::Utc::now(),
            last_interaction_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
        };

        // Upsert to database
        self.feature_repo.upsert_user_features(user_id, &features).await?;

        Ok(features)
    }

    /// Compute genre affinity scores (simplified TF-IDF)
    async fn compute_genre_affinity(&self, _user_id: i32) -> Result<HashMap<String, f32>> {
        // In production: query ClickHouse for genre watch counts,
        // compute TF-IDF scores, normalize
        // Simplified placeholder returning mock data
        let mut affinity = HashMap::new();
        affinity.insert("action".to_string(), 0.8);
        affinity.insert("comedy".to_string(), 0.6);
        affinity.insert("drama".to_string(), 0.4);
        Ok(affinity)
    }

    /// Get user's favorite genres (top N by affinity)
    fn compute_favorite_genres(&self, affinity: &HashMap<String, f32>) -> serde_json::Value {
        let mut genres: Vec<(&String, &f32)> = affinity.iter().collect();
        genres.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<&str> = genres.iter().take(5).map(|(g, _)| g.as_str()).collect();
        json!(top)
    }
}
