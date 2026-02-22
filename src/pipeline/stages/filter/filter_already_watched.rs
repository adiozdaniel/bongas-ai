use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashSet;

pub struct FilterAlreadyWatchedStage;

#[async_trait]
impl PipelineStage for FilterAlreadyWatchedStage {
    fn name(&self) -> &str {
        "filter_already_watched"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        let watched_ids: HashSet<i32> = context.item_feature_service
            .get_watched_item_ids(user_id)
            .await?
            .into_iter()
            .collect();

        let filtered: Vec<ScoredItem> = input.into_iter()
            .filter(|item| !watched_ids.contains(&item.item_id))
            .collect();

        Ok(filtered)
    }
}
