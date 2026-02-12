use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;
use chrono::Utc;

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
        let now = Utc::now();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            promotion_priority: Option<i32>,
            _promotion_start: Option<chrono::DateTime<Utc>>,
            _promotion_end: Option<chrono::DateTime<Utc>>,
            target_segments: Option<JsonValue>,
            promotion_label: Option<String>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, promotion_priority, promotion_start, promotion_end, target_segments, promotion_label
            FROM item_promotions
            WHERE item_id = ANY($1)
                AND (promotion_start IS NULL OR promotion_start <= $2)
                AND (promotion_end IS NULL OR promotion_end >= $2)
                AND is_active = true
            ORDER BY promotion_priority DESC
            "#,
        )
        .bind(&item_ids)
        .bind(now)
        .fetch_all(context.db_pool.as_ref())
        .await
        .unwrap_or_default();

        // Get user segment if needed
        let user_segment: Option<String> = if params.target_segment.is_some() || context.user_id.is_some() {
            if let Some(user_id) = context.user_id {
                #[derive(sqlx::FromRow)]
                struct UserRow {
                    segment: Option<String>,
                }
                sqlx::query_as::<_, UserRow>(
                    "SELECT segment FROM user_profiles WHERE user_id = $1"
                )
                .bind(user_id)
                .fetch_optional(context.db_pool.as_ref())
                .await
                .ok()
                .flatten()
                .and_then(|u| u.segment)
            } else {
                None
            }
        } else {
            None
        };

        let promotion_map: HashMap<i32, (i32, Option<String>)> = rows
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
