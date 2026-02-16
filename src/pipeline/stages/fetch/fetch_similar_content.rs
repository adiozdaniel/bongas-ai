use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashSet;

#[derive(Deserialize)]
struct Params {
    /// Source item ID to find similar content for
    source_item_id: i32,
    /// Number of similar items to fetch
    #[serde(default = "default_limit")]
    limit: usize,
    /// Similarity method: "genre", "embedding", "collaborative", "hybrid"
    #[serde(default = "default_method")]
    method: String,
    /// Minimum similarity score (0.0-1.0)
    #[serde(default = "default_min_sim")]
    min_similarity: f32,
}

fn default_limit() -> usize {
    20
}

fn default_method() -> String {
    "hybrid".to_string()
}

fn default_min_sim() -> f32 {
    0.3
}

pub struct FetchSimilarContentStage;

#[async_trait]
impl PipelineStage for FetchSimilarContentStage {
    fn name(&self) -> &str {
        "fetch_similar_content"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::Empty
    }

    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Get source item features
        let source_item_features_map = context.item_feature_service
            .get_item_features_batch(&[params.source_item_id])
            .await?;

        let source = match source_item_features_map.get(&params.source_item_id) {
            Some(s) => s.clone(), // Clone to own the data for further processing
            None => return Ok(Vec::new()),
        };

        let source_genres: Vec<String> = source.genres
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let source_creators: Vec<String> = source.creators
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Note: ItemFeatureRow's `embedding` is `Option<Vec<f32>>`, not `Option<JsonValue>`
        let source_embedding: Vec<f32> = source.embedding
            .unwrap_or_default();

        let mut items: Vec<ScoredItem> = Vec::new();

        match params.method.as_str() {
            "embedding" if !source_embedding.is_empty() => {
                // Use pre-computed similar items from embedding similarity
                let similar = context.item_feature_service
                    .get_item_similarities_by_embedding(
                        params.source_item_id,
                        params.limit as i64,
                    )
                    .await?;

                for row in similar {
                    if row.similarity_score >= params.min_similarity {
                        items.push(ScoredItem {
                            item_id: row.similar_item_id,
                            score: row.similarity_score,
                            metadata: json!({
                                "source": "similar_content",
                                "method": "embedding",
                                "source_item_id": params.source_item_id,
                                "similarity": row.similarity_score,
                            }),
                        });
                    }
                }
            }
            "collaborative" => {
                // Find items that users who watched source also watched
                let co_watched = context.item_feature_service.get_co_watched_items(
                    params.source_item_id,
                    params.limit as i64,
                ).await?;

                let max_count = co_watched.first().map(|r| r.1).unwrap_or(1) as f32;

                for (item_id, co_watch_count) in co_watched {
                    let similarity = co_watch_count as f32 / max_count;
                    if similarity >= params.min_similarity {
                        items.push(ScoredItem {
                            item_id,
                            score: similarity,
                            metadata: json!({
                                "source": "similar_content",
                                "method": "collaborative",
                                "source_item_id": params.source_item_id,
                                "co_watch_count": co_watch_count,
                            }),
                        });
                    }
                }
            }
            _ => {
                // Genre-based or hybrid similarity
                if source_genres.is_empty() {
                    return Ok(Vec::new());
                }

                let candidates = context.item_feature_service.get_items_by_overlap(
                    params.source_item_id,
                    &source_genres,
                    &source_creators,
                    (params.limit * 3) as i64,
                ).await?;

                for row in candidates {
                    let item_genres: Vec<String> = row.genres.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    let item_creators: Vec<String> = row.creators.as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    // Calculate genre similarity (Jaccard)
                    let genre_set: HashSet<_> = source_genres.iter().map(|g| g.to_lowercase()).collect();
                    let item_set: HashSet<_> = item_genres.iter().map(|g| g.to_lowercase()).collect();
                    let intersection = genre_set.intersection(&item_set).count();
                    let union = genre_set.union(&item_set).count();
                    let genre_sim = if union > 0 { intersection as f32 / union as f32 } else { 0.0 };

                    // Calculate creator overlap
                    let creator_set: HashSet<_> = source_creators.iter().map(|c| c.to_lowercase()).collect();
                    let item_creator_set: HashSet<_> = item_creators.iter().map(|c| c.to_lowercase()).collect();
                    let creator_overlap = creator_set.intersection(&item_creator_set).count();
                    let creator_sim = if !creator_set.is_empty() {
                        creator_overlap as f32 / creator_set.len() as f32
                    } else {
                        0.0
                    };

                    // Hybrid similarity
                    let similarity = genre_sim * 0.7 + creator_sim * 0.3;

                    if similarity >= params.min_similarity {
                        items.push(ScoredItem {
                            item_id: row.item_id,
                            score: similarity * row.popularity_score.unwrap_or(0.5),
                            metadata: json!({
                                "source": "similar_content",
                                "method": params.method,
                                "source_item_id": params.source_item_id,
                                "genre_similarity": genre_sim,
                                "creator_similarity": creator_sim,
                            }),
                        });
                    }
                }

                // Sort by score and limit
                items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                items.truncate(params.limit);
            }
        }

        Ok(items)
    }
}
