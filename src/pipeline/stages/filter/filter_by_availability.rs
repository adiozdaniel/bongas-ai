use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Deserialize)]
struct Params {
    /// Filter by availability status: "available", "coming_soon", "expired", "all"
    #[serde(default = "default_status")]
    status: String,
    /// Include items available within N days (for coming_soon)
    #[serde(default)]
    within_days: Option<i32>,
    /// Whether to include items with unknown availability
    #[serde(default)]
    include_unknown: bool,
}

fn default_status() -> String {
    "available".to_string()
}

pub struct FilterByAvailabilityStage;

#[async_trait]
impl PipelineStage for FilterByAvailabilityStage {
    fn name(&self) -> &str {
        "filter_by_availability"
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
            available_from: Option<chrono::DateTime<Utc>>,
            available_until: Option<chrono::DateTime<Utc>>,
            is_active: Option<bool>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, available_from, available_until, is_active
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(&item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let now = Utc::now();
        let within_days_duration = params.within_days.map(|d| chrono::Duration::days(d as i64));

        let availability_map: HashMap<i32, (bool, bool, bool)> = rows
            .into_iter()
            .map(|row| {
                let is_active = row.is_active.unwrap_or(true);
                let started = row.available_from.map_or(true, |from| now >= from);
                let not_expired = row.available_until.map_or(true, |until| now <= until);

                let is_available = is_active && started && not_expired;
                let is_coming_soon = is_active && !started && row.available_from.is_some();
                let is_expired = row.available_until.map_or(false, |until| now > until);

                // Check if coming soon within specified days
                let coming_soon_in_range = if let (Some(from), Some(within)) = (row.available_from, within_days_duration) {
                    from <= now + within
                } else {
                    true
                };

                (row.item_id, (is_available, is_coming_soon && coming_soon_in_range, is_expired))
            })
            .collect();

        let filtered: Vec<ScoredItem> = input
            .into_iter()
            .filter(|item| {
                match availability_map.get(&item.item_id) {
                    Some((is_available, is_coming_soon, is_expired)) => {
                        match params.status.as_str() {
                            "available" => *is_available,
                            "coming_soon" => *is_coming_soon,
                            "expired" => *is_expired,
                            "all" => true,
                            _ => *is_available,
                        }
                    }
                    None => params.include_unknown,
                }
            })
            .collect();

        Ok(filtered)
    }
}
