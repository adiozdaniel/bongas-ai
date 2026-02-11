use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use chrono::Utc;

#[derive(Deserialize)]
struct Params {
    /// Number of days to consider as "new"
    #[serde(default = "default_days")]
    days: i32,
    /// Maximum items to fetch
    #[serde(default = "default_limit")]
    limit: usize,
    /// Content type filter: "movie", "series", "all"
    #[serde(default = "default_type")]
    content_type: String,
    /// Genre filter (optional)
    #[serde(default)]
    genre: Option<String>,
    /// Sort by: "release_date", "added_date", "popularity"
    #[serde(default = "default_sort")]
    sort_by: String,
}

fn default_days() -> i32 {
    30
}

fn default_limit() -> usize {
    50
}

fn default_type() -> String {
    "all".to_string()
}

fn default_sort() -> String {
    "release_date".to_string()
}

pub struct FetchNewReleasesStage;

#[async_trait]
impl PipelineStage for FetchNewReleasesStage {
    fn name(&self) -> &str {
        "fetch_new_releases"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let cutoff_date = Utc::now() - chrono::Duration::days(params.days as i64);

        let order_clause = match params.sort_by.as_str() {
            "added_date" => "added_date DESC NULLS LAST",
            "popularity" => "popularity_score DESC NULLS LAST",
            _ => "release_date DESC NULLS LAST",
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            title: Option<String>,
            release_date: Option<chrono::DateTime<Utc>>,
            added_date: Option<chrono::DateTime<Utc>>,
            content_type: Option<String>,
            genres: Option<JsonValue>,
            popularity_score: Option<f32>,
        }

        let mut query = format!(
            r#"
            SELECT item_id, title, release_date, added_date, content_type, genres, popularity_score
            FROM item_features
            WHERE is_active = true
                AND (release_date >= $1 OR added_date >= $1)
            "#
        );

        // Add content type filter
        if params.content_type != "all" {
            query.push_str(&format!(" AND content_type = '{}'", params.content_type));
        }

        // Add genre filter
        if let Some(ref genre) = params.genre {
            query.push_str(&format!(" AND genres @> '[\"{}\"]'::jsonb", genre));
        }

        query.push_str(&format!(" ORDER BY {} LIMIT $2", order_clause));

        let rows: Vec<Row> = sqlx::query_as(&query)
            .bind(cutoff_date)
            .bind(params.limit as i64)
            .fetch_all(context.db_pool.as_ref())
            .await?;

        let now = Utc::now();
        let items: Vec<ScoredItem> = rows
            .into_iter()
            .map(|row| {
                // Score based on recency and popularity
                let age_days = row.release_date
                    .or(row.added_date)
                    .map(|d| (now - d).num_days())
                    .unwrap_or(params.days as i64) as f32;

                let recency_score = 1.0 - (age_days / params.days as f32).min(1.0);
                let popularity = row.popularity_score.unwrap_or(0.5);
                let score = recency_score * 0.6 + popularity * 0.4;

                ScoredItem {
                    item_id: row.item_id,
                    score,
                    metadata: json!({
                        "source": "new_releases",
                        "title": row.title,
                        "release_date": row.release_date.map(|d| d.to_rfc3339()),
                        "added_date": row.added_date.map(|d| d.to_rfc3339()),
                        "content_type": row.content_type,
                        "genres": row.genres,
                        "recency_score": recency_score,
                    }),
                }
            })
            .collect();

        Ok(items)
    }
}
