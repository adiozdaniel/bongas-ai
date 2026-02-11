use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use std::collections::HashMap;

/// Maximal Marginal Relevance (MMR) diversification algorithm
/// Balances relevance with diversity by penalizing items similar to already selected items

#[derive(Deserialize)]
struct Params {
    /// Lambda parameter (0.0 = max diversity, 1.0 = max relevance)
    #[serde(default = "default_lambda")]
    lambda: f32,
    /// Feature to use for similarity: "genre", "embedding", "combined"
    #[serde(default = "default_feature")]
    similarity_feature: String,
    /// Maximum number of items to select
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_lambda() -> f32 {
    0.7
}

fn default_feature() -> String {
    "genre".to_string()
}

fn default_limit() -> usize {
    50
}

pub struct DiversifyMMRStage;

impl DiversifyMMRStage {
    /// Calculate Jaccard similarity between two genre sets
    fn jaccard_similarity(genres_a: &[String], genres_b: &[String]) -> f32 {
        if genres_a.is_empty() || genres_b.is_empty() {
            return 0.0;
        }

        let set_a: std::collections::HashSet<_> = genres_a.iter()
            .map(|g| g.to_lowercase())
            .collect();
        let set_b: std::collections::HashSet<_> = genres_b.iter()
            .map(|g| g.to_lowercase())
            .collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Calculate cosine similarity between two embedding vectors
    fn cosine_similarity(emb_a: &[f32], emb_b: &[f32]) -> f32 {
        if emb_a.len() != emb_b.len() || emb_a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = emb_a.iter().zip(emb_b.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = emb_a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = emb_b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

#[async_trait]
impl PipelineStage for DiversifyMMRStage {
    fn name(&self) -> &str {
        "diversify_mmr"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        if input.is_empty() || params.limit == 0 {
            return Ok(input);
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();

        // Fetch features for similarity calculation
        let feature_map: HashMap<i32, (Vec<String>, Vec<f32>)> = match params.similarity_feature.as_str() {
            "embedding" | "combined" => {
                #[derive(sqlx::FromRow)]
                struct Row {
                    item_id: i32,
                    genres: Option<JsonValue>,
                    embedding: Option<JsonValue>,
                }

                let rows: Vec<Row> = sqlx::query_as(
                    "SELECT item_id, genres, embedding FROM item_features WHERE item_id = ANY($1)"
                )
                .bind(&item_ids)
                .fetch_all(context.db_pool.as_ref())
                .await?;

                rows.into_iter()
                    .map(|row| {
                        let genres: Vec<String> = row.genres
                            .and_then(|v| serde_json::from_value(v).ok())
                            .unwrap_or_default();
                        let embedding: Vec<f32> = row.embedding
                            .and_then(|v| serde_json::from_value(v).ok())
                            .unwrap_or_default();
                        (row.item_id, (genres, embedding))
                    })
                    .collect()
            }
            _ => {
                #[derive(sqlx::FromRow)]
                struct Row {
                    item_id: i32,
                    genres: Option<JsonValue>,
                }

                let rows: Vec<Row> = sqlx::query_as(
                    "SELECT item_id, genres FROM item_features WHERE item_id = ANY($1)"
                )
                .bind(&item_ids)
                .fetch_all(context.db_pool.as_ref())
                .await?;

                rows.into_iter()
                    .map(|row| {
                        let genres: Vec<String> = row.genres
                            .and_then(|v| serde_json::from_value(v).ok())
                            .unwrap_or_default();
                        (row.item_id, (genres, Vec::new()))
                    })
                    .collect()
            }
        };

        // Normalize scores for MMR calculation
        let max_score = input.iter().map(|i| i.score).fold(0.0f32, f32::max);
        let input_normalized: Vec<(ScoredItem, f32)> = input
            .into_iter()
            .map(|item| {
                let norm_score = if max_score > 0.0 { item.score / max_score } else { 0.0 };
                (item, norm_score)
            })
            .collect();

        // MMR selection
        let mut selected: Vec<ScoredItem> = Vec::new();
        let mut remaining: Vec<(ScoredItem, f32)> = input_normalized;

        while selected.len() < params.limit && !remaining.is_empty() {
            let mut best_idx = 0;
            let mut best_mmr = f32::NEG_INFINITY;

            for (idx, (candidate, relevance)) in remaining.iter().enumerate() {
                // Calculate max similarity to already selected items
                let max_sim = if selected.is_empty() {
                    0.0
                } else {
                    let candidate_features = feature_map.get(&candidate.item_id);
                    selected.iter()
                        .filter_map(|sel| {
                            let sel_features = feature_map.get(&sel.item_id)?;
                            let candidate_features = candidate_features?;

                            let sim = match params.similarity_feature.as_str() {
                                "embedding" => Self::cosine_similarity(
                                    &candidate_features.1,
                                    &sel_features.1,
                                ),
                                "combined" => {
                                    let genre_sim = Self::jaccard_similarity(
                                        &candidate_features.0,
                                        &sel_features.0,
                                    );
                                    let emb_sim = Self::cosine_similarity(
                                        &candidate_features.1,
                                        &sel_features.1,
                                    );
                                    (genre_sim + emb_sim) / 2.0
                                }
                                _ => Self::jaccard_similarity(
                                    &candidate_features.0,
                                    &sel_features.0,
                                ),
                            };
                            Some(sim)
                        })
                        .fold(0.0f32, f32::max)
                };

                // MMR score = λ * relevance - (1 - λ) * max_similarity
                let mmr = params.lambda * relevance - (1.0 - params.lambda) * max_sim;

                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_idx = idx;
                }
            }

            let (mut best_item, _) = remaining.remove(best_idx);
            best_item.metadata["mmr_score"] = serde_json::json!(best_mmr);
            selected.push(best_item);
        }

        Ok(selected)
    }
}
