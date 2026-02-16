use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
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

    fn input_type(&self) -> StageDataKind {
        StageDataKind::Empty
    }

    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    fn can_parallelize(&self) -> bool {
        true
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
                match context.item_feature_service.get_recent_watch_with_completion(user_id, params.min_completion).await? {
                    Some(id) => id,
                    None => return Ok(Vec::new()),
                }
            }
        };

        // Get source item details
        let source_features = context.item_feature_service.get_item_features_batch(&[source_item_id]).await?;
        let source = match source_features.get(&source_item_id) {
            Some(s) => s,
            None => return Ok(Vec::new()),
        };

        let source_genres: Vec<String> = source.genres.as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let source_creators: Vec<String> = source.creators.as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // Get user's watched items to exclude
        let watched_items: HashSet<i32> = if params.exclude_watched {
            let watched = context.item_feature_service.get_watched_item_ids(user_id).await
                .unwrap_or_default();
            watched.into_iter().collect()
        } else {
            HashSet::new()
        };

        // Find similar items
        let candidates = context.item_feature_service.get_items_by_overlap(
            source_item_id,
            &source_genres,
            &source_creators,
            (params.limit * 3) as i64,
        ).await?;

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
                let item_genres: Vec<String> = row.genres.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let item_creators: Vec<String> = row.creators.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
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
