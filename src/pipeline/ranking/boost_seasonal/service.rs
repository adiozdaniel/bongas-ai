use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use chrono::{Utc, Datelike};

#[derive(Deserialize)]
struct Params {
    /// Boost factor for seasonally relevant content
    #[serde(default = "default_boost")]
    boost_factor: f32,
    /// Override current season (optional): "spring", "summer", "fall", "winter"
    #[serde(default)]
    override_season: Option<String>,
    /// Include holiday-specific boosts
    #[serde(default = "default_true")]
    include_holidays: bool,
}

fn default_boost() -> f32 {
    1.5
}

fn default_true() -> bool {
    true
}

pub struct BoostSeasonalStage;

impl BoostSeasonalStage {
    fn get_current_season() -> &'static str {
        let month = Utc::now().month();
        match month {
            3..=5 => "spring",
            6..=8 => "summer",
            9..=11 => "fall",
            _ => "winter",
        }
    }

    fn get_current_holidays() -> Vec<&'static str> {
        let now = Utc::now();
        let month = now.month();
        let day = now.day();

        let mut holidays = Vec::new();

        // Check for major holidays (approximate dates)
        match (month, day) {
            (12, 20..=31) | (1, 1..=2) => holidays.push("christmas"),
            (10, 25..=31) => holidays.push("halloween"),
            (2, 10..=14) => holidays.push("valentines"),
            (11, 20..=30) => holidays.push("thanksgiving"),
            (7, 1..=7) => holidays.push("independence_day"),
            (3, 14..=17) => holidays.push("st_patricks"),
            _ => {}
        }

        // Season-specific themes
        match month {
            6..=8 => holidays.push("summer_vacation"),
            12 | 1 | 2 => holidays.push("winter_holidays"),
            _ => {}
        }

        holidays
    }
}

#[async_trait]
impl PipelineStage for BoostSeasonalStage {
    fn name(&self) -> &str {
        "boost_seasonal"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let current_season: &str = match &params.override_season {
            Some(s) => s.as_str(),
            None => Self::get_current_season(),
        };

        let current_holidays = if params.include_holidays {
            Self::get_current_holidays()
        } else {
            Vec::new()
        };

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        let boosted: Vec<ScoredItem> = input
            .into_iter()
            .map(|mut item| {
                if let Some(row) = item_features.get(&item.item_id) {
                    let mut relevance_score: f32 = 0.0;

                    let seasonal_tags: Vec<String> = row.seasonal_tags.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let holiday_tags: Vec<String> = row.holiday_tags.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let themes: Vec<String> = row.themes.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    // Check season match
                    let season_match = seasonal_tags.iter()
                        .any(|t| t.to_lowercase() == current_season);
                    if season_match {
                        relevance_score += 0.5;
                    }

                    // Check holiday match
                    let holiday_match = holiday_tags.iter()
                        .any(|t| current_holidays.iter().any(|h| t.to_lowercase().contains(h)));
                    if holiday_match {
                        relevance_score += 0.5;
                    }

                    // Check theme relevance (partial match)
                    let theme_match = themes.iter()
                        .any(|t| {
                            let t_lower = t.to_lowercase();
                            t_lower.contains(current_season) ||
                            current_holidays.iter().any(|h| t_lower.contains(h))
                        });
                    if theme_match {
                        relevance_score += 0.25;
                    }

                    if relevance_score > 0.0 {
                        let boost = 1.0 + (params.boost_factor - 1.0) * relevance_score.min(1.0);
                        item.score *= boost;
                        item.metadata["seasonal_relevance"] = serde_json::json!(relevance_score);
                        item.metadata["is_seasonal"] = serde_json::json!(true);
                    }
                }
                item
            })
            .collect();

        Ok(boosted)
    }
}
