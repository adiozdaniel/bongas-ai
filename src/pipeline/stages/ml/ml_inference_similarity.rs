use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    method: String,
    top_k: usize,
}

pub struct MLInferenceSimilarityStage;

#[async_trait]
impl PipelineStage for MLInferenceSimilarityStage {
    fn name(&self) -> &str {
        "ml_inference_similarity"
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
            method = %params.method,
            input_count = input.len(),
            "Running native similarity inference"
        );

        if input.is_empty() {
            return Ok(vec![]);
        }

        // Get embeddings for seed items
        let seed_ids: Vec<i32> = input.iter().map(|i| i.item_id).collect();
        let seed_embeddings = self.get_embeddings(context, &seed_ids).await?;

        // Get candidate items not in seeds
        #[derive(sqlx::FromRow)]
        struct CandidateRow {
            item_id: i32,
            tfidf_vector: Option<JsonValue>,
        }

        let candidates: Vec<CandidateRow> = sqlx::query_as(
            r#"
            SELECT item_id, tfidf_vector
            FROM item_features
            WHERE item_id != ALL($1)
                AND tfidf_vector IS NOT NULL
            ORDER BY trending_score DESC
            LIMIT 500
            "#,
        )
        .bind(&seed_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        // Score candidates by average similarity to all seeds
        let mut results: Vec<ScoredItem> = Vec::new();

        for cand in candidates {
            let cand_emb: Vec<f32> = cand.tfidf_vector
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_else(|| vec![0.0; 128]);

            let avg_sim: f32 = seed_embeddings.iter()
                .map(|seed_emb| {
                    match params.method.as_str() {
                        "cosine" => cosine_similarity(seed_emb, &cand_emb),
                        "dot_product" => dot_product(seed_emb, &cand_emb),
                        _ => cosine_similarity(seed_emb, &cand_emb),
                    }
                })
                .sum::<f32>() / seed_embeddings.len().max(1) as f32;

            results.push(ScoredItem {
                item_id: cand.item_id,
                score: avg_sim,
                metadata: json!({
                    "similarity_method": params.method,
                    "inference_engine": "native",
                }),
            });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(params.top_k);

        Ok(results)
    }
}

impl MLInferenceSimilarityStage {
    async fn get_embeddings(
        &self,
        context: &ExecutionContext,
        item_ids: &[i32],
    ) -> Result<Vec<Vec<f32>>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tfidf_vector: Option<JsonValue>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT tfidf_vector FROM item_features WHERE item_id = ANY($1)",
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|r| {
            r.tfidf_vector
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_else(|| vec![0.0; 128])
        }).collect())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 { dot / (norm_a * norm_b) } else { 0.0 }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
