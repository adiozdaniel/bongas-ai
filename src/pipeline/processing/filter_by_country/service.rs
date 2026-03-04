use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

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

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        // Check if blocked
                        if let Some(ref blocked_json) = row.blocked_countries {
                            if let Ok(blocked_list) = serde_json::from_value::<Vec<String>>(blocked_json.clone()) {
                                if blocked_list.iter().any(|c| c.to_uppercase() == country_code) {
                                    return false;
                                }
                            }
                        }
                        // Check if available
                        match row.available_countries {
                            Some(ref available_json) => {
                                if let Ok(available_list) = serde_json::from_value::<Vec<String>>(available_json.clone()) {
                                    if available_list.is_empty() {
                                        params.include_unrestricted
                                    } else {
                                        available_list.iter().any(|c| c.to_uppercase() == country_code)
                                    }
                                } else {
                                    params.include_unrestricted
                                }
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
