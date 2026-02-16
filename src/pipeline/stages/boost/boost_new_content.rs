use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use chrono::Utc;

#[derive(Deserialize)]
struct Params {
    /// Boost factor for new content (e.g., 1.8 = 80% boost)
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Content is considered "new" if released within this many days
    #[serde(default = "default_days")]
    new_threshold_days: i32,
    /// Decay rate - how quickly the boost decreases (0.0-1.0)
    #[serde(default = "default_decay")]
    decay_rate: f32,
}

fn default_boost() -> f32 {
    1.8
}

fn default_days() -> i32 {
    30
}

fn default_decay() -> f32 {
    0.5
}

pub struct BoostNewContentStage;

#[async_trait]
impl PipelineStage for BoostNewContentStage {
    fn name(&self) -> &str {
        "boost_new_content"
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

        let now = Utc::now();
        let threshold = chrono::Duration::days(params.new_threshold_days as i64);

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some(row) = item_features.get(&item.item_id) {
                    // Use release_date if available, otherwise fall back to added_date
                    if let Some(date) = row.release_date.or(row.added_date) {
                        let age = now - date;
                        if age <= threshold && age.num_days() >= 0 {
                            // Calculate decay based on age
                            let age_ratio = age.num_days() as f32 / params.new_threshold_days as f32;
                            let decay = (-params.decay_rate * age_ratio).exp();
                            let boost = 1.0 + (params.boost_factor - 1.0) * decay;

                            item.score *= boost;
                            item.metadata["is_new"] = serde_json::json!(true);
                            item.metadata["days_since_release"] = serde_json::json!(age.num_days());
                        }
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
