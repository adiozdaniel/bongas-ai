use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Minimum duration in minutes (optional)
    #[serde(default)]
    min_minutes: Option<i32>,
    /// Maximum duration in minutes (optional)
    #[serde(default)]
    max_minutes: Option<i32>,
    /// Whether to include items with unknown duration
    #[serde(default)]
    include_unknown: bool,
}

pub struct FilterByDurationStage;

#[async_trait]
impl PipelineStage for FilterByDurationStage {
    fn name(&self) -> &str {
        "filter_by_duration"
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

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        if let Some(duration_seconds) = row.duration_seconds {
                            let duration_minutes = duration_seconds / 60;
                            let above_min = params.min_minutes.is_none_or(|min| duration_minutes >= min);
                            let below_max = params.max_minutes.is_none_or(|max| duration_minutes <= max);
                            above_min && below_max
                        } else {
                            params.include_unknown
                        }
                    }
                    _ => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
