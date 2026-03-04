use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Boost factor for high engagement content
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Weight for likes/reactions (0.0-1.0)
    #[serde(default = "default_like_weight")]
    like_weight: f32,
    /// Weight for comments (0.0-1.0)
    #[serde(default = "default_comment_weight")]
    comment_weight: f32,
    /// Weight for shares (0.0-1.0)
    #[serde(default = "default_share_weight")]
    share_weight: f32,
    /// Weight for saves/watchlist adds (0.0-1.0)
    #[serde(default = "default_save_weight")]
    save_weight: f32,
    /// Minimum engagement score to apply boost
    #[serde(default)]
    min_engagement_score: f32,
}

fn default_boost() -> f32 {
    1.6
}

fn default_like_weight() -> f32 {
    0.3
}

fn default_comment_weight() -> f32 {
    0.25
}

fn default_share_weight() -> f32 {
    0.25
}

fn default_save_weight() -> f32 {
    0.2
}

pub struct BoostEngagementStage;

#[async_trait]
impl PipelineStage for BoostEngagementStage {
    fn name(&self) -> &str {
        "boost_engagement"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        // Calculate engagement rates (normalized by view count)
        let engagement_map: HashMap<i32, (f32, i64, i64, i64, i64)> = item_features
            .into_iter()
            .map(|(item_id, row)| {
                let views = row.view_count.max(1) as f32;
                let likes = row.like_count as i64;
                let comments = row.comment_count.unwrap_or(0);
                let shares = row.share_count.unwrap_or(0);
                let saves = row.save_count.unwrap_or(0);

                // Calculate normalized engagement rate
                let like_rate = (likes as f32 / views).min(1.0);
                let comment_rate = (comments as f32 / views).min(1.0);
                let share_rate = (shares as f32 / views).min(1.0);
                let save_rate = (saves as f32 / views).min(1.0);

                let engagement_score =
                    like_rate * params.like_weight +
                    comment_rate * params.comment_weight +
                    share_rate * params.share_weight +
                    save_rate * params.save_weight;

                (item_id, (engagement_score, likes, comments, shares, saves))
            })
            .collect();

        // Find max engagement for normalization
        let max_engagement = engagement_map
            .values()
            .map(|(score, _, _, _, _)| *score)
            .fold(0.0f32, f32::max);

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some((engagement_score, likes, comments, shares, saves)) = engagement_map.get(&item.item_id) {
                    if *engagement_score >= params.min_engagement_score && max_engagement > 0.0 {
                        let normalized = engagement_score / max_engagement;
                        let boost = 1.0 + (params.boost_factor - 1.0) * normalized;

                        item.score *= boost;
                        item.metadata["engagement_score"] = serde_json::json!(engagement_score);
                        item.metadata["engagement_stats"] = serde_json::json!({
                            "likes": likes,
                            "comments": comments,
                            "shares": shares,
                            "saves": saves,
                        });
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
