use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use chrono::{Utc, Datelike};

#[derive(Deserialize)]
struct Params {
    /// Maximum items to fetch
    #[serde(default = "default_limit")]
    limit: usize,
    /// Override season: "spring", "summer", "fall", "winter"
    #[serde(default)]
    season_override: Option<String>,
    /// Include holiday content
    #[serde(default = "default_true")]
    include_holidays: bool,
    /// Minimum popularity score
    #[serde(default)]
    min_popularity: f32,
}

fn default_limit() -> usize {
    30
}

fn default_true() -> bool {
    true
}

pub struct FetchSeasonalContentStage;

impl FetchSeasonalContentStage {
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

        match (month, day) {
            (12, 1..=31) => holidays.push("christmas"),
            (10, 15..=31) => holidays.push("halloween"),
            (2, 1..=14) => holidays.push("valentines"),
            (11, 15..=30) => holidays.push("thanksgiving"),
            (7, 1..=7) => holidays.push("independence"),
            _ => {}
        }

        holidays
    }
}

#[async_trait]
impl PipelineStage for FetchSeasonalContentStage {
    fn name(&self) -> &str {
        "fetch_seasonal_content"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let current_season: &str = match &params.season_override {
            Some(s) => s.as_str(),
            None => Self::get_current_season(),
        };

        let holidays = if params.include_holidays {
            Self::get_current_holidays()
        } else {
            Vec::new()
        };

        // Build search tags
        let mut search_tags: Vec<String> = vec![current_season.to_string()];
        search_tags.extend(holidays.iter().map(|h| h.to_string()));

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            title: Option<String>,
            seasonal_tags: Option<JsonValue>,
            holiday_tags: Option<JsonValue>,
            _themes: Option<JsonValue>,
            popularity_score: Option<f32>,
        }

        // Query for content matching any seasonal/holiday tags
        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, title, seasonal_tags, holiday_tags, themes, popularity_score
            FROM item_features
            WHERE is_active = true
                AND (
                    seasonal_tags ?| $1
                    OR holiday_tags ?| $1
                    OR themes ?| $1
                )
                AND (popularity_score >= $2 OR popularity_score IS NULL)
            ORDER BY popularity_score DESC NULLS LAST
            LIMIT $3
            "#,
        )
        .bind(&search_tags)
        .bind(params.min_popularity)
        .bind(params.limit as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let items: Vec<ScoredItem> = rows
            .into_iter()
            .map(|row| {
                let seasonal_tags: Vec<String> = row.seasonal_tags
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                let holiday_tags: Vec<String> = row.holiday_tags
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                // Calculate relevance score
                let season_match = seasonal_tags.iter()
                    .any(|t| t.to_lowercase() == current_season);
                let holiday_match = holiday_tags.iter()
                    .any(|t| holidays.iter().any(|h| t.to_lowercase().contains(h)));

                let relevance = match (season_match, holiday_match) {
                    (true, true) => 1.0,
                    (true, false) | (false, true) => 0.7,
                    (false, false) => 0.4,
                };

                let popularity = row.popularity_score.unwrap_or(0.5);
                let score = relevance * 0.6 + popularity * 0.4;

                ScoredItem {
                    item_id: row.item_id,
                    score,
                    metadata: json!({
                        "source": "seasonal_content",
                        "title": row.title,
                        "current_season": current_season,
                        "matched_season": season_match,
                        "matched_holiday": holiday_match,
                        "seasonal_tags": seasonal_tags,
                        "holiday_tags": holiday_tags,
                    }),
                }
            })
            .collect();

        Ok(items)
    }
}
