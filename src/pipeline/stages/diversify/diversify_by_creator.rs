use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Maximum items per creator/director
    #[serde(default = "default_max")]
    max_per_creator: usize,
    /// Field to use for creator: "director", "creator", "studio", "actor"
    #[serde(default = "default_field")]
    creator_field: String,
    /// Whether to consider multiple creators per item
    #[serde(default = "default_true")]
    consider_all_creators: bool,
}

fn default_max() -> usize {
    2
}

fn default_field() -> String {
    "creator".to_string()
}

fn default_true() -> bool {
    true
}

pub struct DiversifyByCreatorStage;

#[async_trait]
impl PipelineStage for DiversifyByCreatorStage {
    fn name(&self) -> &str {
        "diversify_by_creator"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        // Build query based on creator field
        let field_name = match params.creator_field.as_str() {
            "director" => "directors",
            "studio" => "studios",
            "actor" => "actors",
            _ => "creators",
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            creators: Option<JsonValue>,
        }

        let query = format!(
            "SELECT item_id, {} as creators FROM item_features WHERE item_id = ANY($1)",
            field_name
        );

        let rows: Vec<Row> = sqlx::query_as(&query)
            .bind(&item_ids)
            .fetch_all(context.db_pool.as_ref())
            .await?;

        let creator_map: HashMap<i32, Vec<String>> = rows
            .into_iter()
            .map(|row| {
                let creators: Vec<String> = row.creators
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                (row.item_id, creators)
            })
            .collect();

        // Track how many items we've included per creator
        let mut creator_counts: HashMap<String, usize> = HashMap::new();
        let mut selected: Vec<ScoredItem> = Vec::new();

        for item in input {
            let creators = creator_map
                .get(&item.item_id)
                .cloned()
                .unwrap_or_default();

            if creators.is_empty() {
                // No creator info, include by default
                selected.push(item);
                continue;
            }

            // Check if any creator is at limit
            let creators_to_check = if params.consider_all_creators {
                creators.clone()
            } else {
                creators.first().cloned().into_iter().collect()
            };

            let all_under_limit = creators_to_check.iter().all(|creator| {
                creator_counts.get(&creator.to_lowercase()).unwrap_or(&0) < &params.max_per_creator
            });

            if all_under_limit {
                // Increment counts for all creators
                for creator in &creators_to_check {
                    *creator_counts.entry(creator.to_lowercase()).or_insert(0) += 1;
                }
                selected.push(item);
            }
        }

        Ok(selected)
    }
}
