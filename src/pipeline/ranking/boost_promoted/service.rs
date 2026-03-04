use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Boost factor for promoted content
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Maximum number of promoted items to boost
    #[serde(default = "default_max")]
    max_promoted_items: usize,
    /// Only include promotions targeting this user segment (optional)
    #[serde(default)]
    target_segment: Option<String>,
    /// Mark promoted items in metadata
    #[serde(default = "default_true")]
    mark_as_promoted: bool,
}

fn default_boost() -> f32 {
    2.0
}

fn default_max() -> usize {
    5
}

fn default_true() -> bool {
    true
}

pub struct BoostPromotedStage;

#[async_trait]
impl PipelineStage for BoostPromotedStage {
    fn name(&self) -> &str {
        "boost_promoted"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let promotions = context.item_feature_service.get_promotions_batch(&item_ids).await
            .unwrap_or_default();

        // Get user segment if needed
        let user_segment: Option<String> = if params.target_segment.is_some() || context.user_id.is_some() {
            if let Some(user_id) = context.user_id {
                context.item_feature_service.get_user_profile(user_id).await
                    .ok()
                    .flatten()
                    .and_then(|p| p.segment)
            } else {
                None
            }
        } else {
            None
        };

        let promotion_map: HashMap<i32, (i32, Option<String>)> = promotions
            .into_iter()
            .filter(|row| {
                // Check segment targeting
                if let Some(ref target) = params.target_segment {
                    if let Some(ref segments) = row.target_segments {
                        if let Ok(segments_list) = serde_json::from_value::<Vec<String>>(segments.clone()) {
                            if !segments_list.iter().any(|s| s.to_lowercase() == target.to_lowercase()) {
                                return false;
                            }
                        }
                    }
                }

                // Check user segment matching
                if let Some(ref user_seg) = user_segment {
                    if let Some(ref segments) = row.target_segments {
                        if let Ok(segments_list) = serde_json::from_value::<Vec<String>>(segments.clone()) {
                            if !segments_list.is_empty() && !segments_list.iter().any(|s| s.to_lowercase() == user_seg.to_lowercase()) {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .map(|row| {
                (row.item_id, (row.promotion_priority.unwrap_or(0), row.promotion_label))
            })
            .collect();

        let mut promoted_count = 0;
        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if promoted_count < params.max_promoted_items {
                    if let Some((priority, label)) = promotion_map.get(&item.item_id) {
                        // Boost based on priority (higher priority = more boost)
                        let priority_factor = (*priority as f32 / 100.0).min(1.0);
                        let boost = 1.0 + (params.boost_factor - 1.0) * (0.5 + 0.5 * priority_factor);

                        item.score *= boost;
                        promoted_count += 1;

                        if params.mark_as_promoted {
                            item.metadata["is_promoted"] = serde_json::json!(true);
                            item.metadata["promotion_priority"] = serde_json::json!(priority);
                            if let Some(lbl) = label {
                                item.metadata["promotion_label"] = serde_json::json!(lbl);
                            }
                        }
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
