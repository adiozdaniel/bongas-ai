use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind, CompactMetadata};
use crate::pipeline::context::ExecutionContext;


#[derive(Deserialize)]
struct Params {
    limit: usize,
    min_views: Option<i32>,
}

pub struct FetchPopularContentStage;

#[async_trait]
impl PipelineStage for FetchPopularContentStage {
    fn name(&self) -> &str {
        "fetch_popular_content"
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
        let min_views = params.min_views.unwrap_or(0);

        // Thunder-Lite: Check Hot Registry first
        if let Some(registry) = &context.hot_registry {
            // If we don't have a strict min_views filter (or if hot items satisfy it implicitly),
            // we can serve directly from RAM.
            // HotRegistry stores top-k by trending score, which usually correlates with views.
            let hot_items = registry.get_top_k(params.limit);
            
            if !hot_items.is_empty() {
                // tracing::info!(count = hot_items.len(), "Hot Registry hit (Thunder-Lite)");
                return Ok(hot_items.into_iter().map(|item| {
                    ScoredItem::new(
                        item.item_id,
                        item.score,
                        item.metadata,
                    )
                }).collect());
            }
        }

        // Record ClickHouse query
        // if let Some(analytics) = context.analytics() {
        //     analytics.record_clickhouse_query("fetch_popular_content", "item_features");
        // }

        // Start timing the query
        // let _timer = if let Some(analytics) = context.analytics() {
        //     Some(analytics.start_clickhouse_query_timer("fetch_popular_content"))
        // } else {
        //     None
        // };

        let popular_items = context.item_feature_service
            .get_popular_content(min_views, params.limit as i64)
            .await?;

        let items: Vec<ScoredItem> = popular_items.into_iter().map(|row| {
            let mut item = ScoredItem::new(
                row.item_id,
                row.trending_score,
                json!({
                    "view_count": row.view_count,
                    "completion_rate": row.completion_rate,
                    "age_rating": row.age_rating,
                    "published_at": row.published_at,
                    "genres": row.genres,
                }),
            );

            // Phase 6: Populate Zero-Copy Fast Metadata
            let compact = CompactMetadata {
                features: vec![row.trending_score, row.completion_rate],
                flags: if row.is_explicit.unwrap_or(false) { 1 } else { 0 },
                category_id: 0, // Placeholder
            };
            if let Ok(bytes) = rkyv::to_bytes::<_, 256>(&compact) {
                item.fast_metadata = Some(bytes.to_vec());
            }

            item
        }).collect();

        Ok(items)
    }
}
