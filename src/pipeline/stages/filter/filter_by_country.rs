use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Country code to check availability for (e.g., "US", "GB", "DE")
    /// If not provided, uses context.location
    #[serde(default)]
    country_code: Option<String>,
    /// Whether to include items with no country restrictions
    #[serde(default = "default_true")]
    include_unrestricted: bool,
}

fn default_true() -> bool {
    true
}

pub struct FilterByCountryStage;

#[async_trait]
impl PipelineStage for FilterByCountryStage {
    fn name(&self) -> &str {
        "filter_by_country"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Get country from params or context
        let country_code = params.country_code
            .or_else(|| context.location.clone())
            .unwrap_or_else(|| "US".to_string())
            .to_uppercase();

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            available_countries: Option<JsonValue>,
            blocked_countries: Option<JsonValue>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, available_countries, blocked_countries
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let availability_map: HashMap<i32, (Option<Vec<String>>, Option<Vec<String>>)> = rows
            .into_iter()
            .map(|row| {
                let available: Option<Vec<String>> = row.available_countries
                    .and_then(|v| serde_json::from_value(v).ok());
                let blocked: Option<Vec<String>> = row.blocked_countries
                    .and_then(|v| serde_json::from_value(v).ok());
                (row.item_id, (available, blocked))
            })
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match availability_map.get(&item.item_id) {
                    Some((available, blocked)) => {
                        // Check if blocked
                        if let Some(blocked_list) = blocked {
                            if blocked_list.iter().any(|c| c.to_uppercase() == country_code) {
                                return false;
                            }
                        }
                        // Check if available
                        match available {
                            Some(available_list) if !available_list.is_empty() => {
                                available_list.iter().any(|c| c.to_uppercase() == country_code)
                            }
                            _ => params.include_unrestricted,
                        }
                    }
                    None => params.include_unrestricted,
                }
            })
            .collect();

        Ok(filtered)
    }
}
