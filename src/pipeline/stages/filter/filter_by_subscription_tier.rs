use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

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
            #[derive(sqlx::FromRow)]
            struct UserRow {
                subscription_tier: Option<String>,
            }
            let user: Option<UserRow> = sqlx::query_as(
                "SELECT subscription_tier FROM user_profiles WHERE user_id = $1"
            )
            .bind(user_id)
            .fetch_optional(context.db_pool.as_ref())
            .await?;
            user.and_then(|u| u.subscription_tier).unwrap_or_else(|| "free".to_string())
        } else {
            "free".to_string()
        };

        let user_level = Self::tier_to_level(&user_tier);
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            required_tier: Option<String>,
            is_free: Option<bool>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, required_tier, is_free
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let tier_map: HashMap<i32, (i32, bool)> = rows
            .into_iter()
            .map(|row| {
                let required_level = row.required_tier
                    .map(|t| Self::tier_to_level(&t))
                    .unwrap_or(0);
                let is_free = row.is_free.unwrap_or(required_level == 0);
                (row.item_id, (required_level, is_free))
            })
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter_map(|mut item| {
                match tier_map.get(&item.item_id) {
                    Some((required_level, is_free)) => {
                        // Always include free content if configured
                        if *is_free && params.include_free_content {
                            return Some(item);
                        }

                        // Check if user has access
                        if user_level >= *required_level {
                            return Some(item);
                        }

                        // Optionally show as locked instead of filtering
                        if params.show_locked {
                            item.metadata["locked"] = serde_json::json!(true);
                            item.metadata["required_tier"] = serde_json::json!(
                                match *required_level {
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
