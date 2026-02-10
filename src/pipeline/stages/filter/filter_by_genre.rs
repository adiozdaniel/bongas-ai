use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    genres: Vec<String>,
    mode: String,
}

pub struct FilterByGenreStage;

#[async_trait]
impl PipelineStage for FilterByGenreStage {
    fn name(&self) -> &str {
        "filter_by_genre"
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
            genres: Option<JsonValue>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, genres
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let genre_map: HashMap<i32, Vec<String>> = rows
            .into_iter()
            .filter_map(|row| {
                let genres: Option<Vec<String>> = row.genres
                    .and_then(|g| serde_json::from_value(g).ok());
                genres.map(|g| (row.item_id, g))
            })
            .collect();

        let filtered: Vec<ScoredItem> = input.into_iter()
            .filter(|item| {
                if let Some(item_genres) = genre_map.get(&item.item_id) {
                    let has_genre = item_genres.iter().any(|g| params.genres.contains(g));
                    match params.mode.as_str() {
                        "include" => has_genre,
                        "exclude" => !has_genre,
                        _ => true,
                    }
                } else {
                    true
                }
            })
            .collect();

        Ok(filtered)
    }
}
