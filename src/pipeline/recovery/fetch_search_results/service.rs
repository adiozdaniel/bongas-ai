use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use tracing::debug;

#[derive(Deserialize)]
struct Params {
    /// The search index to query (defaults to config)
    pub index: Option<String>,
    /// Number of results to fetch
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

pub struct FetchSearchResultsStage;

#[async_trait]
impl PipelineStage for FetchSearchResultsStage {
    fn name(&self) -> &str {
        "fetch_search_results"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        
        // Extract query from metadata
        // In the stage API, 'q' is passed in the context metadata
        let query = context.experiment_overrides.get("q")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query.is_empty() {
            debug!("Empty search query, returning no results");
            return Ok(Vec::new());
        }

        let client = context.search_client.as_ref()
            .ok_or_else(|| anyhow!("Meilisearch client not initialized in ExecutionContext"))?;

        let index_name = params.index.as_ref()
            .cloned()
            .unwrap_or_else(|| "items".to_string());

        let index = client.index(index_name);

        // Execute search
        let search_result = index.search()
            .with_query(query)
            .with_limit(params.limit)
            .execute::<JsonValue>()
            .await?;

        let mut items = Vec::new();

        for hit in search_result.hits {
            let item_id = hit.result.get("item_id")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);

            if let Some(id) = item_id {
                // Meilisearch doesn't always provide a normalized score in the same way
                // In a production setup, we'd use the '_rankingScore' if enabled
                let score = 1.0; // Placeholder for now, re-ranking will handle weights

                items.push(ScoredItem::new(
                    id,
                    score,
                    json!({
                        "source": "meilisearch",
                        "query": query,
                        "hit_metadata": hit.result,
                    }),
                ));
            }
        }

        debug!(query = %query, count = items.len(), "Meilisearch recovery completed");

        Ok(items)
    }
}
