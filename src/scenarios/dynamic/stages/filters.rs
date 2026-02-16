use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;

#[derive(Deserialize)]
struct FilterConfig {
    r#type: String,
    value: JsonValue,
}

#[derive(Deserialize)]
struct Params {
    filters: Vec<FilterConfig>,
}

pub struct DynamicFiltersStage;

#[async_trait]
impl PipelineStage for DynamicFiltersStage {
    fn name(&self) -> &str { "dynamic_filters" }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let stage_config: Params = serde_json::from_value(params.clone())
            .context("Failed to parse dynamic_filters params")?;

        if input.is_empty() || stage_config.filters.is_empty() {
            return Ok(input);
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered = input.into_iter().filter(|item| {
            if let Some(features) = item_features.get(&item.item_id) {
                for filter in &stage_config.filters {
                    match filter.r#type.as_str() {
                        "genre" => {
                            if let Some(genre) = filter.value.as_str() {
                                let item_genres = features.genres.as_ref()
                                    .and_then(|g| g.as_array())
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                                    .unwrap_or_default();
                                if !item_genres.contains(&genre) {
                                    return false;
                                }
                            }
                        }
                        "age_rating" => {
                            if let Some(rating) = filter.value.as_str() {
                                if features.age_rating.as_deref() != Some(rating) {
                                    return false;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        }).collect();

        Ok(filtered)
    }
}
