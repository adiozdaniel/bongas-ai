use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    weight: f32,
}

pub struct BoostByPopularityStage;

#[async_trait]
impl PipelineStage for BoostByPopularityStage {
    fn name(&self) -> &str {
        "boost_by_popularity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            trending_score: f32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, trending_score
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let pop_map: HashMap<i32, f32> = rows
            .into_iter()
            .map(|row| (row.item_id, row.trending_score))
            .collect();

        let boosted: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            if let Some(&pop_score) = pop_map.get(&item.item_id) {
                item.score = item.score * (1.0 - params.weight) + pop_score * params.weight;
            }
            item
        }).collect();

        Ok(boosted)
    }
}
