use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    max_same_genre: usize,
}

pub struct DiversifyGenresStage;

#[async_trait]
impl PipelineStage for DiversifyGenresStage {
    fn name(&self) -> &str {
        "diversify_genres"
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

        let mut genre_counts: HashMap<String, usize> = HashMap::new();
        let mut diversified: Vec<ScoredItem> = Vec::new();

        for item in input {
            if let Some(genres) = genre_map.get(&item.item_id) {
                let primary_genre = genres.first();
                if let Some(genre) = primary_genre {
                    let count = genre_counts.entry(genre.clone()).or_insert(0);
                    if *count < params.max_same_genre {
                        *count += 1;
                        diversified.push(item);
                    }
                } else {
                    diversified.push(item);
                }
            } else {
                diversified.push(item);
            }
        }

        Ok(diversified)
    }
}
