use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind, CompactMetadata};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Limit number of items per preferred genre
    #[serde(default = "default_per_genre")]
    items_per_genre: usize,
    /// Total item limit
    #[serde(default = "default_limit")]
    limit: usize,
    /// Minimum affinity score to consider a genre preferred
    #[serde(default = "default_min_affinity")]
    min_affinity: f32,
    /// Include items from watchlist
    #[serde(default = "default_true")]
    include_watchlist: bool,
}

fn default_per_genre() -> usize {
    10
}

fn default_limit() -> usize {
    50
}

fn default_min_affinity() -> f32 {
    0.3
}

fn default_true() -> bool {
    true
}

pub struct FetchUserPreferencesStage;

#[async_trait]
impl PipelineStage for FetchUserPreferencesStage {
    fn name(&self) -> &str {
        "fetch_user_preferences"
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

        let user_id = match context.user_id {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // Get user's genre preferences
        let user_features = context.item_feature_service.get_user_features(user_id).await?;

        let genre_affinity: Vec<(String, f32)> = user_features
            .and_then(|p| p.genre_affinity)
            .and_then(|v| serde_json::from_value::<std::collections::HashMap<String, f32>>(v).ok())
            .map(|m| {
                let mut vec: Vec<_> = m.into_iter()
                    .filter(|(_, score)| *score >= params.min_affinity)
                    .collect();
                vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                vec
            })
            .unwrap_or_default();

        if genre_affinity.is_empty() {
            return Ok(Vec::new());
        }

        let mut items: Vec<ScoredItem> = Vec::new();

        // Fetch items for each preferred genre
        for (genre, affinity) in genre_affinity.iter().take(5) {
            let genre_items = context.item_feature_service.get_items_by_genre(
                json!([genre]),
                params.items_per_genre as i64,
            ).await?;

            for item in genre_items {
                let base_score = item.popularity_score.unwrap_or(0.5);
                let score = base_score * affinity;
                let mut scored_item = ScoredItem::new(
                    item.item_id,
                    score,
                    json!({
                        "source": "user_preferences",
                        "matched_genre": genre,
                        "genre_affinity": affinity,
                        "title": item.title,
                        "age_rating": item.age_rating,
                        "published_at": item.published_at.or(item.release_date).map(|d| d.to_rfc3339()),
                        "genres": item.genres,
                    }),
                );

                // Phase 6: Populate Zero-Copy Fast Metadata
                let compact = CompactMetadata {
                    features: vec![score, *affinity],
                    flags: 0,
                    category_id: 0,
                };
                if let Ok(bytes) = rkyv::to_bytes::<_, 256>(&compact) {
                    scored_item.fast_metadata = Some(bytes.to_vec());
                }

                items.push(scored_item);
            }
        }

        // Optionally include watchlist items
        if params.include_watchlist {
            let watchlist = context.item_feature_service.get_user_watchlist(user_id, 20).await
                .unwrap_or_default();

            for wl_item in watchlist {
                let mut scored_item = ScoredItem::new(
                    wl_item.item_id,
                    1.0, // High score for watchlist items
                    json!({
                        "source": "watchlist",
                        "added_at": wl_item.added_at.to_rfc3339(),
                    }),
                );

                // Phase 6: Populate Zero-Copy Fast Metadata
                let compact = CompactMetadata {
                    features: vec![1.0, 0.0],
                    flags: 0,
                    category_id: 0,
                };
                if let Ok(bytes) = rkyv::to_bytes::<_, 256>(&compact) {
                    scored_item.fast_metadata = Some(bytes.to_vec());
                }

                items.push(scored_item);
            }
        }

        // Deduplicate and limit
        let mut seen = std::collections::HashSet::new();
        items.retain(|item| seen.insert(item.item_id));
        items.truncate(params.limit);

        Ok(items)
    }
}
