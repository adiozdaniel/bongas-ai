use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Minimum release year (optional)
    #[serde(default)]
    min_year: Option<i32>,
    /// Maximum release year (optional)
    #[serde(default)]
    max_year: Option<i32>,
    /// Specific years to include (optional, overrides min/max)
    #[serde(default)]
    years: Option<Vec<i32>>,
    /// Whether to include items with unknown release year
    #[serde(default)]
    include_unknown: bool,
}

pub struct FilterByReleaseYearStage;

#[async_trait]
impl PipelineStage for FilterByReleaseYearStage {
    fn name(&self) -> &str {
        "filter_by_release_year"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        if let Some(year) = row.release_year {
                            // If specific years are provided, check against those
                            if let Some(ref years) = params.years {
                                return years.contains(&year);
                            }
                            // Otherwise check min/max range
                            let above_min = params.min_year.map_or(true, |min| year >= min);
                            let below_max = params.max_year.map_or(true, |max| year <= max);
                            above_min && below_max
                        } else {
                            params.include_unknown
                        }
                    }
                    _ => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
