use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashSet;

#[derive(Deserialize)]
struct Params {
    /// Source item ID (the item the user watched)
    /// If not provided, uses user's most recently watched item
    #[serde(default)]
    source_item_id: Option<i32>,
    /// Maximum items to fetch
    #[serde(default = "default_limit")]
    limit: usize,
    /// Minimum completion rate of source item to trigger recommendations
    #[serde(default = "default_min_completion")]
    min_completion: f32,
    /// Exclude items user has already watched
    #[serde(default = "default_true")]
    exclude_watched: bool,
}

fn default_limit() -> usize {
    20
}

fn default_min_completion() -> f32 {
    0.5
}

fn default_true() -> bool {
    true
}

pub struct FetchBecauseYouWatchedStage;

#[async_trait]
impl PipelineStage for FetchBecauseYouWatchedStage {
    fn name(&self) -> &str {
        "fetch_because_you_watched"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // Determine source item
        let source_item_id = match params.source_item_id {
            Some(id) => id,
            None => {
                // Get user's most recently watched item with good completion
                #[derive(sqlx::FromRow)]
                struct RecentWatch {
                    item_id: i32,
                }

                let recent: Option<RecentWatch> = sqlx::query_as(
                    r#"
                    SELECT item_id
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND completion_rate >= $2
                    ORDER BY created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(user_id)
                .bind(params.min_completion)
                .fetch_optional(context.db_pool.as_ref())
                .await?;

                match recent {
                    Some(r) => r.item_id,
                    None => return Ok(Vec::new()),
                }
            }
        };

        // Get source item details
        #[derive(sqlx::FromRow)]
        struct SourceItem {
            title: Option<String>,
            genres: Option<JsonValue>,
            creators: Option<JsonValue>,
        }

        let source: Option<SourceItem> = sqlx::query_as(
            "SELECT title, genres, creators FROM item_features WHERE item_id = $1"
        )
        .bind(source_item_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        let source = match source {
            Some(s) => s,
            None => return Ok(Vec::new()),
        };

        let source_genres: Vec<String> = source.genres
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let source_creators: Vec<String> = source.creators
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Get user's watched items to exclude
        let watched_items: HashSet<i32> = if params.exclude_watched {
            #[derive(sqlx::FromRow)]
            struct WatchedRow {
                item_id: i32,
            }

            let watched: Vec<WatchedRow> = sqlx::query_as(
                "SELECT DISTINCT item_id FROM user_interactions WHERE user_id = $1 AND interaction_type = 'view'"
            )
            .bind(user_id)
            .fetch_all(context.db_pool.as_ref())
            .await
            .unwrap_or_default();

            watched.into_iter().map(|w| w.item_id).collect()
        } else {
            HashSet::new()
        };

        // Find similar items
        #[derive(sqlx::FromRow)]
        struct SimilarRow {
            item_id: i32,
            title: Option<String>,
            genres: Option<JsonValue>,
            creators: Option<JsonValue>,
            popularity_score: Option<f32>,
        }

        let candidates: Vec<SimilarRow> = sqlx::query_as(
            r#"
            SELECT item_id, title, genres, creators, popularity_score
            FROM item_features
            WHERE item_id != $1
                AND is_active = true
                AND (genres && $2::jsonb OR creators && $3::jsonb)
            ORDER BY popularity_score DESC NULLS LAST
            LIMIT $4
            "#,
        )
        .bind(source_item_id)
        .bind(json!(source_genres))
        .bind(json!(source_creators))
        .bind((params.limit * 3) as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let source_genre_set: HashSet<_> = source_genres.iter()
            .map(|g| g.to_lowercase())
            .collect();

        let source_creator_set: HashSet<_> = source_creators.iter()
            .map(|c| c.to_lowercase())
            .collect();

        let mut items: Vec<ScoredItem> = candidates
            .into_iter()
            .filter(|row| !watched_items.contains(&row.item_id))
            .map(|row| {
                let item_genres: Vec<String> = row.genres
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                let item_creators: Vec<String> = row.creators
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                // Calculate similarity
                let item_genre_set: HashSet<_> = item_genres.iter()
                    .map(|g| g.to_lowercase())
                    .collect();
                let genre_overlap = source_genre_set.intersection(&item_genre_set).count();
                let genre_sim = if !source_genre_set.is_empty() {
                    genre_overlap as f32 / source_genre_set.len() as f32
                } else {
                    0.0
                };

                let item_creator_set: HashSet<_> = item_creators.iter()
                    .map(|c| c.to_lowercase())
                    .collect();
                let creator_overlap = source_creator_set.intersection(&item_creator_set).count();
                let creator_sim = if !source_creator_set.is_empty() {
                    creator_overlap as f32 / source_creator_set.len() as f32
                } else {
                    0.0
                };

                let similarity = genre_sim * 0.6 + creator_sim * 0.4;
                let popularity = row.popularity_score.unwrap_or(0.5);
                let score = similarity * 0.7 + popularity * 0.3;

                ScoredItem {
                    item_id: row.item_id,
                    score,
                    metadata: json!({
                        "source": "because_you_watched",
                        "source_item_id": source_item_id,
                        "source_title": source.title,
                        "title": row.title,
                        "genre_similarity": genre_sim,
                        "creator_similarity": creator_sim,
                    }),
                }
            })
            .collect();

        // Sort by score and limit
        items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        items.truncate(params.limit);

        Ok(items)
    }
}
