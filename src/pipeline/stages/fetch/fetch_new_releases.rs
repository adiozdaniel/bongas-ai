use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
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

    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let cutoff_date = Utc::now() - chrono::Duration::days(params.days as i64);

        let rows = context.item_feature_service.get_new_releases_advanced(
            cutoff_date,
            &params.content_type,
            params.genre.clone(),
            &params.sort_by,
            params.limit as i64,
        ).await?;

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

                ScoredItem::new(
                    row.item_id,
                    score,
                    json!({
                        "source": "new_releases",
                        "title": row.title,
                        "release_date": row.release_date.map(|d| d.to_rfc3339()),
                        "added_date": row.added_date.map(|d| d.to_rfc3339()),
                        "published_at": row.published_at.or(row.added_date).or(row.release_date).map(|d| d.to_rfc3339()),
                        "age_rating": row.age_rating,
                        "content_type": row.content_type,
                        "genres": row.genres,
                        "recency_score": recency_score,
                    }),
                )
            })
            .collect();

        Ok(items)
    }
}
