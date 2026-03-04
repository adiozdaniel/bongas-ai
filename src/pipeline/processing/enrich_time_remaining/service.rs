use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use std::collections::HashMap;


pub struct EnrichTimeRemainingStage;

#[async_trait]
impl PipelineStage for EnrichTimeRemainingStage {
    fn name(&self) -> &str {
        "enrich_time_remaining"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    fn can_parallelize(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features_map = context.item_feature_service
            .get_item_features_batch(item_ids.as_slice())
            .await?;

        let duration_map: HashMap<i32, i32> = item_features_map
            .into_iter()
            .filter_map(|(item_id, features)| features.duration_seconds.map(|d| (item_id, d)))
            .collect();

        let enriched: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            if let Some(&duration) = duration_map.get(&item.item_id) {
                if let Some(completion) = item.metadata.get("completion").and_then(|v| v.as_f64()) {
                    let watched_seconds = (duration as f64 * completion) as i32;
                    let remaining_seconds = duration - watched_seconds;

                    item.metadata["duration_seconds"] = json!(duration);
                    item.metadata["watched_seconds"] = json!(watched_seconds);
                    item.metadata["remaining_seconds"] = json!(remaining_seconds);
                }
            }
            item
        }).collect();

        Ok(enriched)
    }
}
