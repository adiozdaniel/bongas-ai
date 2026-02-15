use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;


#[derive(Deserialize)]
struct Params {
    limit: usize,
    hours: i32,
}

pub struct FetchRecentWatchesStage;

#[async_trait]
impl PipelineStage for FetchRecentWatchesStage {
    fn name(&self) -> &str {
        "fetch_recent_watches"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        let recent_watches = context.item_feature_service
            .get_recent_watches(user_id, params.hours, params.limit as i64)
            .await?;

        let items: Vec<ScoredItem> = recent_watches.into_iter().map(|row| {
            ScoredItem {
                item_id: row.item_id,
                score: 1.0,
                metadata: json!({
                    "last_watched": row.last_watched.map(|t| t.to_rfc3339()),
                }),
            }
        }).collect();

        Ok(items)
    }
}
