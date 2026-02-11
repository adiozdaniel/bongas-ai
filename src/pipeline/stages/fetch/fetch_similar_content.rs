use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
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

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        // Get source item features
        #[derive(sqlx::FromRow)]
        struct SourceItem {
            genres: Option<JsonValue>,
            creators: Option<JsonValue>,
            embedding: Option<JsonValue>,
            content_type: Option<String>,
        }

        let source: Option<SourceItem> = sqlx::query_as(
            "SELECT genres, creators, embedding, content_type FROM item_features WHERE item_id = $1"
        )
        .bind(params.source_item_id)
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

        let source_embedding: Vec<f32> = source.embedding
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let mut items: Vec<ScoredItem> = Vec::new();

        match params.method.as_str() {
            "embedding" if !source_embedding.is_empty() => {
                // Use pre-computed similar items from embedding similarity
                #[derive(sqlx::FromRow)]
                struct SimilarRow {
                    similar_item_id: i32,
                    similarity_score: f32,
                }

                let similar: Vec<SimilarRow> = sqlx::query_as(
                    r#"
                    SELECT similar_item_id, similarity_score
                    FROM item_similarities
                    WHERE item_id = $1 AND similarity_type = 'embedding'
                    ORDER BY similarity_score DESC
                    LIMIT $2
                    "#,
                )
                .bind(params.source_item_id)
                .bind(params.limit as i64)
                .fetch_all(context.db_pool.as_ref())
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
                #[derive(sqlx::FromRow)]
                struct CoWatchedRow {
                    item_id: i32,
                    co_watch_count: i64,
                }

                let co_watched: Vec<CoWatchedRow> = sqlx::query_as(
                    r#"
                    SELECT ui2.item_id, COUNT(DISTINCT ui2.user_id) as co_watch_count
                    FROM user_interactions ui1
                    JOIN user_interactions ui2 ON ui1.user_id = ui2.user_id
                    WHERE ui1.item_id = $1
                        AND ui2.item_id != $1
                        AND ui1.interaction_type = 'view'
                        AND ui2.interaction_type = 'view'
                    GROUP BY ui2.item_id
                    ORDER BY co_watch_count DESC
                    LIMIT $2
                    "#,
                )
                .bind(params.source_item_id)
                .bind(params.limit as i64)
                .fetch_all(context.db_pool.as_ref())
                .await?;

                let max_count = co_watched.first().map(|r| r.co_watch_count).unwrap_or(1) as f32;

                for row in co_watched {
                    let similarity = row.co_watch_count as f32 / max_count;
                    if similarity >= params.min_similarity {
                        items.push(ScoredItem {
                            item_id: row.item_id,
                            score: similarity,
                            metadata: json!({
                                "source": "similar_content",
                                "method": "collaborative",
                                "source_item_id": params.source_item_id,
                                "co_watch_count": row.co_watch_count,
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

                #[derive(sqlx::FromRow)]
                struct GenreMatchRow {
                    item_id: i32,
                    genres: Option<JsonValue>,
                    creators: Option<JsonValue>,
                    popularity_score: Option<f32>,
                }

                let candidates: Vec<GenreMatchRow> = sqlx::query_as(
                    r#"
                    SELECT item_id, genres, creators, popularity_score
                    FROM item_features
                    WHERE item_id != $1
                        AND genres && $2::jsonb
                        AND is_active = true
                    ORDER BY popularity_score DESC NULLS LAST
                    LIMIT $3
                    "#,
                )
                .bind(params.source_item_id)
                .bind(json!(source_genres))
                .bind((params.limit * 3) as i64) // Fetch more to filter later
                .fetch_all(context.db_pool.as_ref())
                .await?;

                for row in candidates {
                    let item_genres: Vec<String> = row.genres
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_default();

                    let item_creators: Vec<String> = row.creators
                        .and_then(|v| serde_json::from_value(v).ok())
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
