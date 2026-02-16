use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct BoosterConfig {
    r#type: String,
    weight: f32,
}

#[derive(Deserialize)]
struct Params {
    boosters: Vec<BoosterConfig>,
}

pub struct DynamicBoostersStage;

#[async_trait]
impl PipelineStage for DynamicBoostersStage {
    fn name(&self) -> &str { "dynamic_boosters" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let stage_config: Params = serde_json::from_value(params.clone())
            .context("Failed to parse dynamic_boosters params")?;

        if input.is_empty() || stage_config.boosters.is_empty() {
            return Ok(input);
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let mut boosted = input;

        for booster in stage_config.boosters {
            for item in boosted.iter_mut() {
                if let Some(features) = item_features.get(&item.item_id) {
                    let boost_val = match booster.r#type.as_str() {
                        "popularity" => features.trending_score,
                        "recency" => features.release_year.map(|y| (y as f32 - 2000.0) / 25.0).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    item.score = item.score * (1.0 - booster.weight) + boost_val * booster.weight;
                }
            }
        }

        Ok(boosted)
    }
}
