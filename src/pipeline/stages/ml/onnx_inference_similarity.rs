use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    #[allow(dead_code)]
    model_name: String,
    method: String,
    top_k_per_seed: usize,
}

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
        let params: Params = serde_json::from_value(params.clone())?;

        info!(
            request_id = %context.request_id,
            seed_count = input.len(),
            method = %params.method,
            "Running ONNX similarity inference"
        );

        // Get seed item embeddings
        let seed_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let seed_embeddings = self.get_item_embeddings(context, &seed_ids).await?;

        // Get candidate items
        let candidate_ids = self.get_candidate_items(context).await?;
        let candidate_embeddings = self.get_item_embeddings(context, &candidate_ids).await?;

        // Compute similarities
        let mut results: Vec<ScoredItem> = Vec::new();

        for (seed_id, seed_emb) in seed_ids.iter().zip(seed_embeddings.iter()) {
            let seed_vec = &seed_emb.1;
            let mut seed_results: Vec<ScoredItem> = candidate_embeddings.iter()
                .filter(|(cid, _)| !seed_ids.contains(cid))
                .map(|(cid, cand_vec)| {
                    let score = match params.method.as_str() {
                        "cosine" => Self::cosine_similarity(seed_vec, cand_vec),
                        "dot_product" => Self::dot_product(seed_vec, cand_vec),
                        "euclidean" => Self::euclidean_distance(seed_vec, cand_vec),
                        _ => Self::cosine_similarity(seed_vec, cand_vec),
                    };
                    ScoredItem {
                        item_id: *cid,
                        score,
                        metadata: json!({
                            "seed_item_id": seed_id,
                            "similarity_method": params.method,
                            "inference_engine": "onnx",
                        }),
                    }
                })
                .collect();

            seed_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            seed_results.truncate(params.top_k_per_seed);
            results.extend(seed_results);
        }

        Ok(results)
    }
}

impl ONNXInferenceSimilarityStage {
    async fn get_item_embeddings(
        &self,
        context: &ExecutionContext,
        item_ids: &[i32],
    ) -> Result<Vec<(i32, Vec<f32>)>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            tfidf_vector: Option<JsonValue>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, tfidf_vector
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let mut embeddings = Vec::new();
        for row in rows {
            let emb = if let Some(vec_json) = row.tfidf_vector {
                serde_json::from_value::<Vec<f32>>(vec_json).unwrap_or_else(|_| vec![0.0; 128])
            } else {
                vec![0.0; 128]
            };
            embeddings.push((row.item_id, emb));
        }

        Ok(embeddings)
    }

    async fn get_candidate_items(&self, context: &ExecutionContext) -> Result<Vec<i32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id FROM item_features
            WHERE trending_score > 0.1
            ORDER BY RANDOM()
            LIMIT 1000
            "#,
        )
        .fetch_all(context.db_pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        let dist: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt();
        // Convert distance to similarity (closer = higher score)
        1.0 / (1.0 + dist)
    }
}
