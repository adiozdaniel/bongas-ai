use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use std::collections::HashMap;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::{info, debug};

#[derive(Deserialize)]
struct Params {
    /// Model name for embedding extraction (optional - uses precomputed if not specified)
    #[serde(default)]
    model_name: Option<String>,
    /// Similarity method: "cosine", "dot_product", or "euclidean"
    #[serde(default = "default_method")]
    method: String,
    /// Number of similar items to return per seed item
    #[serde(default = "default_top_k_per_seed")]
    top_k_per_seed: usize,
    /// Total maximum results to return
    #[serde(default = "default_max_results")]
    max_results: usize,
    /// Embedding dimension
    #[serde(default = "default_embedding_dim")]
    embedding_dim: usize,
    /// Minimum similarity threshold
    #[serde(default)]
    min_similarity: Option<f32>,
    /// Number of candidate items to consider
    #[serde(default = "default_candidate_limit")]
    candidate_limit: usize,
}

fn default_method() -> String { "cosine".to_string() }
fn default_top_k_per_seed() -> usize { 10 }
fn default_max_results() -> usize { 50 }
fn default_embedding_dim() -> usize { 128 }
fn default_candidate_limit() -> usize { 1000 }

pub struct ONNXInferenceSimilarityStage;

#[async_trait]
impl PipelineStage for ONNXInferenceSimilarityStage {
    fn name(&self) -> &str {
        "onnx_inference_similarity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse onnx_inference_similarity params")?;

        if input.is_empty() {
            debug!(
                request_id = %context.request_id,
                "ONNX similarity skipped: no seed items"
            );
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            seed_count = input.len(),
            method = %params.method,
            model = ?params.model_name,
            "Running ONNX similarity inference"
        );

        let start = std::time::Instant::now();

        // Get seed item embeddings
        let seed_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let seed_embeddings = self
            .get_item_embeddings(context, &seed_ids, params.embedding_dim)
            .await?;

        // Get candidate items (excluding seeds)
        let candidate_ids = self
            .get_candidate_items(context, &seed_ids, params.candidate_limit)
            .await?;
        let candidate_embeddings = self
            .get_item_embeddings(context, &candidate_ids, params.embedding_dim)
            .await?;

        // Compute similarities
        let mut results: Vec<ScoredItem> = Vec::new();

        // Convert seed embeddings to a map for lookup
        let seed_emb_map: HashMap<i32, Vec<f32>> = seed_embeddings.into_iter().collect();
        let cand_emb_map: HashMap<i32, Vec<f32>> = candidate_embeddings.into_iter().collect();

        for &seed_id in &seed_ids {
            let seed_vec = match seed_emb_map.get(&seed_id) {
                Some(v) => v,
                None => continue,
            };

            let mut seed_results: Vec<ScoredItem> = cand_emb_map
                .iter()
                .filter(|(cid, _)| !seed_ids.contains(cid))
                .map(|(cid, cand_vec)| {
                    let score = match params.method.as_str() {
                        "cosine" => Self::cosine_similarity(seed_vec, cand_vec),
                        "dot_product" => Self::dot_product(seed_vec, cand_vec),
                        "euclidean" => Self::euclidean_similarity(seed_vec, cand_vec),
                        _ => Self::cosine_similarity(seed_vec, cand_vec),
                    };
                    ScoredItem {
                        item_id: *cid,
                        score,
                        metadata: json!({
                            "seed_item_id": seed_id,
                            "similarity_method": params.method,
                            "inference_engine": "onnx_similarity"
                        }),
                    }
                })
                .collect();

            // Filter by minimum similarity
            if let Some(min_sim) = params.min_similarity {
                seed_results.retain(|item| item.score >= min_sim);
            }

            // Sort and limit per seed
            seed_results.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
            seed_results.truncate(params.top_k_per_seed);
            results.extend(seed_results);
        }

        // Deduplicate by item_id (keep highest score)
        let mut seen: HashMap<i32, usize> = HashMap::new();
        let mut deduped: Vec<ScoredItem> = Vec::new();

        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        for item in results {
            if !seen.contains_key(&item.item_id) {
                seen.insert(item.item_id, deduped.len());
                deduped.push(item);
            }
        }

        // Limit total results
        deduped.truncate(params.max_results);

        let inference_time = start.elapsed();

        info!(
            request_id = %context.request_id,
            seed_count = seed_ids.len(),
            candidate_count = candidate_ids.len(),
            output_count = deduped.len(),
            inference_ms = inference_time.as_millis() as u64,
            "ONNX similarity inference complete"
        );

        Ok(deduped)
    }
}

impl ONNXInferenceSimilarityStage {
    /// Get item embeddings from database
    async fn get_item_embeddings(
        &self,
        context: &ExecutionContext,
        item_ids: &[i32],
        embedding_dim: usize,
    ) -> Result<Vec<(i32, Vec<f32>)>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            embedding: Option<Vec<f32>>,
            tfidf_vector: Option<JsonValue>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, embedding, tfidf_vector
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let mut embeddings = Vec::new();
        for row in rows {
            let emb = if let Some(embedding) = row.embedding {
                // Prefer pre-computed embedding
                Self::pad_or_truncate(embedding, embedding_dim)
            } else if let Some(vec_json) = row.tfidf_vector {
                // Fall back to TF-IDF vector
                let vec = serde_json::from_value::<Vec<f32>>(vec_json)
                    .unwrap_or_else(|_| vec![0.0; embedding_dim]);
                Self::pad_or_truncate(vec, embedding_dim)
            } else {
                // Cold start
                vec![0.0; embedding_dim]
            };
            embeddings.push((row.item_id, emb));
        }

        // Fill missing items with zeros
        for &item_id in item_ids {
            if !embeddings.iter().any(|(id, _)| *id == item_id) {
                embeddings.push((item_id, vec![0.0; embedding_dim]));
            }
        }

        Ok(embeddings)
    }

    /// Get candidate items for similarity search (excluding seed items)
    async fn get_candidate_items(
        &self,
        context: &ExecutionContext,
        exclude_ids: &[i32],
        limit: usize,
    ) -> Result<Vec<i32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id FROM item_features
            WHERE item_id != ALL($1)
              AND (embedding IS NOT NULL OR tfidf_vector IS NOT NULL)
            ORDER BY trending_score DESC NULLS LAST, view_count DESC NULLS LAST
            LIMIT $2
            "#,
        )
        .bind(exclude_ids)
        .bind(limit as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Pad or truncate vector to exact dimension
    fn pad_or_truncate(mut vec: Vec<f32>, target_dim: usize) -> Vec<f32> {
        if vec.len() < target_dim {
            vec.resize(target_dim, 0.0);
        } else if vec.len() > target_dim {
            vec.truncate(target_dim);
        }
        vec
    }

    /// Cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Dot product between two vectors
    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Euclidean distance converted to similarity (closer = higher score)
    fn euclidean_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dist: f32 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt();
        // Convert distance to similarity
        1.0 / (1.0 + dist)
    }
}
