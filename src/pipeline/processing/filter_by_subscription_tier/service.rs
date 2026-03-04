use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// User's subscription tier: "free", "basic", "premium", "vip"
    /// If not provided, attempts to fetch from user profile
    #[serde(default)]
    user_tier: Option<String>,
    /// Whether to include free content regardless of tier
    #[serde(default = "default_true")]
    include_free_content: bool,
    /// Whether to show premium content as "locked" instead of filtering
    #[serde(default)]
    show_locked: bool,
}

fn default_true() -> bool {
    true
}

pub struct FilterBySubscriptionTierStage;

impl FilterBySubscriptionTierStage {
    fn tier_to_level(tier: &str) -> i32 {
        match tier.to_lowercase().as_str() {
            "free" => 0,
            "basic" => 1,
            "premium" => 2,
            "vip" => 3,
            _ => 0,
        }
    }
}

#[async_trait]
impl PipelineStage for FilterBySubscriptionTierStage {
    fn name(&self) -> &str {
        "filter_by_subscription_tier"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Get user tier from params or fetch from database
        let user_tier = if let Some(tier) = params.user_tier {
            tier
        } else if let Some(user_id) = context.user_id {
            let user = context.item_feature_service.get_user_profile(user_id).await?;
            user.and_then(|u| u.subscription_tier).unwrap_or_else(|| "free".to_string())
        } else {
            "free".to_string()
        };

        let user_level = Self::tier_to_level(&user_tier);
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter_map(|mut item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        let required_level = row.required_tier.as_ref()
                            .map(|t| Self::tier_to_level(t))
                            .unwrap_or(0);
                        let is_free = row.is_free.unwrap_or(required_level == 0);

                        // Always include free content if configured
                        if is_free && params.include_free_content {
                            return Some(item);
                        }

                        // Check if user has access
                        if user_level >= required_level {
                            return Some(item);
                        }

                        // Optionally show as locked instead of filtering
                        if params.show_locked {
                            item.metadata["locked"] = serde_json::json!(true);
                            item.metadata["required_tier"] = serde_json::json!(
                                match required_level {
                                    0 => "free",
                                    1 => "basic",
                                    2 => "premium",
                                    3 => "vip",
                                    _ => "unknown",
                                }
                            );
                            return Some(item);
                        }

                        None
                    }
                    None => Some(item), // No tier requirement, include
                }
            })
            .collect();

        Ok(filtered)
    }
}
