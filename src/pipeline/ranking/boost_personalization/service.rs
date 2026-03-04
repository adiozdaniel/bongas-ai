use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Maximum boost factor for personalized content
    #[serde(default = "default_boost")]
    max_boost_factor: f32,
    /// Model to use for personalization: "two_tower", "collaborative", "content_based"
    #[serde(default = "default_model")]
    model_type: String,
    /// Minimum personalization score to apply boost
    #[serde(default)]
    min_score: f32,
    /// Blend with popularity (0.0 = pure personalization, 1.0 = pure popularity)
    #[serde(default = "default_blend")]
    popularity_blend: f32,
}

fn default_boost() -> f32 {
    2.0
}

fn default_model() -> String {
    "collaborative".to_string()
}

fn default_blend() -> f32 {
    0.2
}

pub struct BoostPersonalizationStage;

#[async_trait]
impl PipelineStage for BoostPersonalizationStage {
    fn name(&self) -> &str {
        "boost_personalization"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(input), // No user context, skip personalization
        };

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        // Fetch pre-computed personalization scores from feature store
        let scores = context.item_feature_service.get_user_item_scores_batch(user_id, &item_ids, &params.model_type).await
            .unwrap_or_default();

        // If no pre-computed scores, try to compute from user features
        let score_map: HashMap<i32, f32> = if scores.is_empty() {
            // Fallback: compute basic collaborative filtering score
            Self::compute_fallback_scores(context, user_id, &item_ids).await?
        } else {
            scores.into_iter()
                .map(|row| (row.item_id, row.score))
                .collect()
        };

        // Get popularity scores for blending
        let popularity_map = if params.popularity_blend > 0.0 {
            Self::get_popularity_scores(context, &item_ids).await?
        } else {
            HashMap::new()
        };

        // Normalize scores
        let max_personal = score_map.values().cloned().fold(0.0f32, f32::max);
        let max_popularity = popularity_map.values().cloned().fold(0.0f32, f32::max);

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                let personal_score = score_map.get(&item.item_id).copied().unwrap_or(0.0);
                let popularity_score = popularity_map.get(&item.item_id).copied().unwrap_or(0.0);

                // Normalize and blend
                let normalized_personal = if max_personal > 0.0 { personal_score / max_personal } else { 0.0 };
                let normalized_popularity = if max_popularity > 0.0 { popularity_score / max_popularity } else { 0.0 };

                let blended_score = normalized_personal * (1.0 - params.popularity_blend)
                    + normalized_popularity * params.popularity_blend;

                if blended_score >= params.min_score {
                    let boost = 1.0 + (params.max_boost_factor - 1.0) * blended_score;
                    item.score *= boost;

                    item.metadata["personalization_score"] = serde_json::json!(normalized_personal);
                    item.metadata["blended_score"] = serde_json::json!(blended_score);
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}

impl BoostPersonalizationStage {
    async fn compute_fallback_scores(
        context: &ExecutionContext,
        user_id: i32,
        item_ids: &[i32],
    ) -> Result<HashMap<i32, f32>> {
        // Get user's genre preferences
        let user = context.item_feature_service.get_user_features(user_id).await?;

        let genre_affinity: HashMap<String, f32> = user
            .and_then(|u| u.genre_affinity)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        if genre_affinity.is_empty() {
            return Ok(HashMap::new());
        }

        // Get item genres
        let item_features = context.item_feature_service.get_item_features_batch(item_ids).await?;

        // Compute content-based scores
        let scores: HashMap<i32, f32> = item_features
            .into_iter()
            .map(|(item_id, row)| {
                let genres: Vec<String> = row.genres
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                let score: f32 = genres.iter()
                    .filter_map(|g| genre_affinity.get(&g.to_lowercase()))
                    .sum::<f32>() / genres.len().max(1) as f32;

                (item_id, score)
            })
            .collect();

        Ok(scores)
    }

    async fn get_popularity_scores(
        context: &ExecutionContext,
        item_ids: &[i32],
    ) -> Result<HashMap<i32, f32>> {
        let item_features = context.item_feature_service.get_item_features_batch(item_ids).await?;

        Ok(item_features.into_iter()
            .filter_map(|(item_id, row)| row.popularity_score.map(|s| (item_id, s)))
            .collect())
    }
}
