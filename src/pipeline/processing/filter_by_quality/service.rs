use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Minimum video quality: "SD", "HD", "FHD", "4K", "8K"
    #[serde(default)]
    min_quality: Option<String>,
    /// Maximum video quality (for bandwidth-limited scenarios)
    #[serde(default)]
    max_quality: Option<String>,
    /// Require HDR support
    #[serde(default)]
    require_hdr: bool,
    /// Require Dolby Vision support
    #[serde(default)]
    require_dolby_vision: bool,
    /// Require Dolby Atmos audio
    #[serde(default)]
    require_dolby_atmos: bool,
    /// Whether to include items with unknown quality
    #[serde(default = "default_true")]
    include_unknown: bool,
}

fn default_true() -> bool {
    true
}

pub struct FilterByQualityStage;

impl FilterByQualityStage {
    fn quality_to_level(quality: &str) -> i32 {
        match quality.to_uppercase().as_str() {
            "SD" | "480P" => 0,
            "HD" | "720P" => 1,
            "FHD" | "1080P" | "FULL HD" => 2,
            "4K" | "2160P" | "UHD" => 3,
            "8K" | "4320P" => 4,
            _ => -1,
        }
    }
}

#[async_trait]
impl PipelineStage for FilterByQualityStage {
    fn name(&self) -> &str {
        "filter_by_quality"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let min_level = params.min_quality.as_ref().map(|q| Self::quality_to_level(q));
        let max_level = params.max_quality.as_ref().map(|q| Self::quality_to_level(q));

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match item_features.get(&item.item_id) {
                    Some(row) => {
                        let quality_level = row.max_resolution.as_ref()
                            .map(|q| Self::quality_to_level(q))
                            .unwrap_or(-1);

                        // Unknown quality
                        if quality_level < 0 {
                            return params.include_unknown;
                        }

                        // Check quality range
                        if let Some(min) = min_level {
                            if quality_level < min {
                                return false;
                            }
                        }
                        if let Some(max) = max_level {
                            if quality_level > max {
                                return false;
                            }
                        }

                        // Check HDR requirements
                        if params.require_hdr && !row.has_hdr.unwrap_or(false) {
                            return false;
                        }
                        if params.require_dolby_vision && !row.has_dolby_vision.unwrap_or(false) {
                            return false;
                        }
                        if params.require_dolby_atmos && !row.has_dolby_atmos.unwrap_or(false) {
                            return false;
                        }

                        true
                    }
                    None => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
