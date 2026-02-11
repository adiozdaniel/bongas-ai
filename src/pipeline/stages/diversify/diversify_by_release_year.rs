use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Params {
    /// Maximum items per decade
    #[serde(default = "default_max")]
    max_per_decade: usize,
    /// Alternatively, maximum items per specific year
    #[serde(default)]
    max_per_year: Option<usize>,
    /// Minimum decades to have represented (if possible)
    #[serde(default = "default_min_decades")]
    min_decades: usize,
    /// Prefer newer content when diversifying
    #[serde(default)]
    prefer_newer: bool,
}

fn default_max() -> usize {
    5
}

fn default_min_decades() -> usize {
    3
}

pub struct DiversifyByReleaseYearStage;

impl DiversifyByReleaseYearStage {
    fn year_to_decade(year: i32) -> i32 {
        (year / 10) * 10
    }
}

#[async_trait]
impl PipelineStage for DiversifyByReleaseYearStage {
    fn name(&self) -> &str {
        "diversify_by_release_year"
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
            release_year: Option<i32>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT item_id, release_year FROM item_features WHERE item_id = ANY($1)"
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let year_map: HashMap<i32, Option<i32>> = rows
            .into_iter()
            .map(|row| (row.item_id, row.release_year))
            .collect();

        // Group items by decade/year
        let use_year = params.max_per_year.is_some();
        let max_per_period = params.max_per_year.unwrap_or(params.max_per_decade);

        let mut period_counts: HashMap<i32, usize> = HashMap::new();
        let mut selected: Vec<ScoredItem> = Vec::new();
        let mut unknown_year: Vec<ScoredItem> = Vec::new();

        // Sort by score (and optionally by year if prefer_newer)
        let mut items_with_years: Vec<(ScoredItem, Option<i32>)> = input
            .into_iter()
            .map(|item| {
                let year = year_map.get(&item.item_id).copied().flatten();
                (item, year)
            })
            .collect();

        if params.prefer_newer {
            items_with_years.sort_by(|a, b| {
                match (a.1, b.1) {
                    (Some(year_a), Some(year_b)) => year_b.cmp(&year_a),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }

        for (item, year) in items_with_years {
            match year {
                Some(y) => {
                    let period = if use_year { y } else { Self::year_to_decade(y) };
                    let count = period_counts.entry(period).or_insert(0);

                    if *count < max_per_period {
                        *count += 1;
                        selected.push(item);
                    }
                }
                None => {
                    unknown_year.push(item);
                }
            }
        }

        // Ensure minimum decades are represented
        let unique_decades = period_counts.keys().count();
        if unique_decades < params.min_decades && !unknown_year.is_empty() {
            // Add some items with unknown years to increase variety
            let to_add = params.min_decades.saturating_sub(unique_decades);
            selected.extend(unknown_year.into_iter().take(to_add));
        } else {
            // Add remaining unknown year items
            selected.extend(unknown_year);
        }

        Ok(selected)
    }
}
