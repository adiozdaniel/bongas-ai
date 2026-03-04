use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use std::collections::{HashMap, HashSet};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Deserialize)]
struct Params {
    /// Percentage of results to replace with serendipitous items (0.0-1.0)
    #[serde(default = "default_ratio")]
    serendipity_ratio: f32,
    /// Minimum score difference from user's typical preferences
    #[serde(default = "default_min_diff")]
    min_preference_diff: f32,
    /// Exclude genres the user has explicitly disliked
    #[serde(default = "default_true")]
    respect_dislikes: bool,
    /// Prefer critically acclaimed content for serendipity
    #[serde(default)]
    prefer_acclaimed: bool,
    /// Minimum rating for serendipitous items
    #[serde(default = "default_min_rating")]
    min_rating: f32,
}

fn default_ratio() -> f32 {
    0.15
}

fn default_min_diff() -> f32 {
    0.3
}

fn default_true() -> bool {
    true
}

fn default_min_rating() -> f32 {
    7.0
}

pub struct SerendipityStage;

#[async_trait]
impl PipelineStage for SerendipityStage {
    fn name(&self) -> &str {
        "serendipity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        if input.is_empty() {
            return Ok(input);
        }

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(input), // No user context, skip serendipity
        };

        // Get user's genre preferences
        let user_features = context.item_feature_service.get_user_features(user_id).await?;

        let genre_affinity: HashMap<String, f32> = user_features.as_ref()
            .and_then(|u| u.genre_affinity.clone())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let disliked_genres: HashSet<String> = user_features
            .and_then(|u| u.disliked_genres.clone())
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
            .map(|v| v.into_iter().map(|g| g.to_lowercase()).collect())
            .unwrap_or_default();

        // Get item features
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        // Identify serendipitous candidates
        let mut candidates: Vec<&ScoredItem> = Vec::new();

        for item in &input {
            if let Some(row) = item_features.get(&item.item_id) {
                let genres: Vec<String> = row.genres.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let rating = row.critic_rating
                    .or(row.user_rating)
                    .unwrap_or(0.0);
                let is_acclaimed = row.is_award_winner.unwrap_or(false) || rating >= 8.0;

                // Check if item is outside user's comfort zone
                let avg_affinity: f32 = genres.iter()
                    .filter_map(|g| genre_affinity.get(&g.to_lowercase()))
                    .sum::<f32>() / genres.len().max(1) as f32;

                let is_serendipitous = avg_affinity < (1.0 - params.min_preference_diff);

                // Check disliked genres
                let has_disliked = params.respect_dislikes && genres.iter()
                    .any(|g| disliked_genres.contains(&g.to_lowercase()));

                // Check rating threshold
                let meets_rating = rating >= params.min_rating;

                // Check acclaimed preference
                let meets_acclaimed = !params.prefer_acclaimed || is_acclaimed;

                if is_serendipitous && !has_disliked && meets_rating && meets_acclaimed {
                    candidates.push(item);
                }
            }
        }

        // Calculate how many serendipitous items to include
        let serendipity_count = (input.len() as f32 * params.serendipity_ratio).ceil() as usize;
        let serendipity_count = serendipity_count.min(candidates.len());

        // Randomly select serendipitous items
        let mut rng = thread_rng();
        candidates.shuffle(&mut rng);
        let serendipitous_ids: HashSet<i32> = candidates
            .iter()
            .take(serendipity_count)
            .map(|item| item.item_id)
            .collect();

        // Build final list: regular items + marked serendipitous items
        let mut result: Vec<ScoredItem> = Vec::new();
        let mut serendipity_added = 0;

        for mut item in input {
            if serendipitous_ids.contains(&item.item_id) {
                item.metadata["is_serendipitous"] = serde_json::json!(true);
                serendipity_added += 1;
            }
            result.push(item);
        }

        // Log serendipity stats
        tracing::debug!(
            serendipity_count = serendipity_added,
            total_items = result.len(),
            "Serendipity stage completed"
        );

        Ok(result)
    }
}
