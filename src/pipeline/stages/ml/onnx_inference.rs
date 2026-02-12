use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::{info, warn, debug};

#[derive(Deserialize)]
#[allow(dead_code)]
struct Params {
    /// Model name to use (e.g., "two_tower", "bert4rec")
    model_name: String,
    /// Model format (for metadata purposes)
    #[serde(default = "default_model_format")]
    model_format: String,
    /// Model version (optional, defaults to latest deployed)
    #[serde(default)]
    model_version: Option<String>,
    /// Maximum number of results to return
    top_k: usize,
    /// Batch size for inference (defaults to 64)
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    /// User feature dimension (must match model input)
    #[serde(default = "default_user_dim")]
    user_feature_dim: usize,
    /// Item feature dimension (must match model input)
    #[serde(default = "default_item_dim")]
    item_feature_dim: usize,
    /// Minimum score threshold (filter out low scores)
    #[serde(default)]
    min_score_threshold: Option<f32>,
}

fn default_model_format() -> String { "onnx".to_string() }
fn default_batch_size() -> usize { 64 }
fn default_user_dim() -> usize { 64 }
fn default_item_dim() -> usize { 32 }

pub struct ONNXInferenceStage;

#[async_trait]
impl PipelineStage for ONNXInferenceStage {
    fn name(&self) -> &str {
        "onnx_inference"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse onnx_inference params")?;

        let user_id = context.user_id
            .ok_or_else(|| anyhow::anyhow!("user_id required for ONNX inference"))?;

        if input.is_empty() {
            debug!(
                request_id = %context.request_id,
                "ONNX inference skipped: no input items"
            );
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            model_name = %params.model_name,
            model_format = %params.model_format,
            input_count = input.len(),
            user_id = user_id,
            "Starting ONNX inference stage"
        );

        let start = std::time::Instant::now();

        // Step 1: Get ONNX model from ModelLoader
        let model = match &params.model_version {
            Some(version) => {
                context.model_loader
                    .get_model_version(&params.model_name, version)
                    .await
                    .with_context(|| format!(
                        "Failed to load model {}:{}",
                        params.model_name, version
                    ))?
            }
            None => {
                context.model_loader
                    .get_model(&params.model_name)
                    .await
                    .with_context(|| format!(
                        "Failed to load latest model {}",
                        params.model_name
                    ))?
            }
        };

        // Step 2: Get user features from database
        let user_features = self
            .get_user_features(context, user_id, params.user_feature_dim)
            .await
            .context("Failed to get user features")?;

        // Step 3: Get item features for all candidates
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = self
            .get_item_features(context, &item_ids, params.item_feature_dim)
            .await
            .context("Failed to get item features")?;

        // Step 4: Prepare batch inputs for ONNX model
        let mut user_batch: Vec<Vec<f32>> = Vec::new();
        let mut item_batch: Vec<Vec<f32>> = Vec::new();
        let mut batch_item_ids: Vec<i32> = Vec::new();

        for item in &input {
            let item_feats = item_features_map
                .get(&item.item_id)
                .cloned()
                .unwrap_or_else(|| vec![0.0; params.item_feature_dim]);

            user_batch.push(user_features.clone());
            item_batch.push(item_feats);
            batch_item_ids.push(item.item_id);
        }

        // Step 5: Run ONNX inference
        let scores = if !user_batch.is_empty() {
            let mut engine = model.write().await;
            engine.predict_batch(user_batch, item_batch)
                .context("ONNX batch inference failed")?
        } else {
            warn!(
                request_id = %context.request_id,
                "No valid features found for ONNX inference"
            );
            vec![0.5; input.len()] // Fallback neutral scores
        };

        let inference_time = start.elapsed();

        // Step 6: Combine scores with items
        let mut results: Vec<ScoredItem> = batch_item_ids
            .iter()
            .zip(scores.iter())
            .map(|(&item_id, &score)| {
                ScoredItem {
                    item_id,
                    score,
                    metadata: json!({
                        "inference_engine": "onnx",
                        "model_name": params.model_name,
                        "model_format": params.model_format,
                        "model_version": params.model_version.as_deref().unwrap_or("latest"),
                        "inference_time_ms": inference_time.as_millis() as u64
                    }),
                }
            })
            .collect();

        // Step 7: Filter by minimum score threshold
        if let Some(min_score) = params.min_score_threshold {
            let before_count = results.len();
            results.retain(|item| item.score >= min_score);
            if results.len() < before_count {
                debug!(
                    request_id = %context.request_id,
                    filtered = before_count - results.len(),
                    min_score = min_score,
                    "Filtered low-scoring items"
                );
            }
        }

        // Step 8: Sort by score descending and limit
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(params.top_k);

        info!(
            request_id = %context.request_id,
            model_name = %params.model_name,
            input_count = input.len(),
            output_count = results.len(),
            inference_ms = inference_time.as_millis() as u64,
            "ONNX inference complete"
        );

        Ok(results)
    }
}

impl ONNXInferenceStage {
    /// Get user features from database with caching
    async fn get_user_features(
        &self,
        context: &ExecutionContext,
        user_id: i32,
        feature_dim: usize,
    ) -> Result<Vec<f32>> {
        // Try Redis cache first
        let cache_key = format!("user_features:{}", user_id);
        if let Ok(Some(cached)) = context.redis.get(&cache_key).await {
            if let Ok(features) = serde_json::from_str::<Vec<f32>>(&cached) {
                debug!(user_id = user_id, "User features from cache");
                return Ok(Self::pad_or_truncate(features, feature_dim));
            }
        }

        // Query database
        #[derive(sqlx::FromRow)]
        struct Row {
            genre_affinity: Option<JsonValue>,
            embedding: Option<Vec<f32>>,
            total_watch_time_minutes: Option<i32>,
            avg_completion_rate: Option<f32>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT genre_affinity, embedding, total_watch_time_minutes, avg_completion_rate
            FROM user_features
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        let features = match row {
            Some(row) => {
                // Prefer pre-computed embedding if available
                if let Some(embedding) = row.embedding {
                    embedding
                } else {
                    // Build features from components
                    let mut features = Vec::new();

                    if let Some(genre_affinity) = row.genre_affinity {
                        if let Ok(affinity) = serde_json::from_value::<Vec<f32>>(genre_affinity) {
                            features.extend(affinity);
                        }
                    }

                    // Normalize numeric features
                    features.push(row.total_watch_time_minutes.unwrap_or(0) as f32 / 10000.0);
                    features.push(row.avg_completion_rate.unwrap_or(0.0));

                    features
                }
            }
            None => {
                // Cold start: return default embedding
                warn!(user_id = user_id, "No user features found, using cold start defaults");
                vec![0.0; feature_dim]
            }
        };

        // Cache for future requests (1 hour TTL)
        if let Ok(json) = serde_json::to_string(&features) {
            let _ = context.redis.set_ex(&cache_key, &json, 3600).await;
        }

        Ok(Self::pad_or_truncate(features, feature_dim))
    }

    /// Get item features for multiple items
    async fn get_item_features(
        &self,
        context: &ExecutionContext,
        item_ids: &[i32],
        feature_dim: usize,
    ) -> Result<std::collections::HashMap<i32, Vec<f32>>> {
        use std::collections::HashMap;

        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            embedding: Option<Vec<f32>>,
            tfidf_vector: Option<JsonValue>,
            view_count: Option<i32>,
            trending_score: Option<f32>,
            completion_rate: Option<f32>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, embedding, tfidf_vector, view_count, trending_score, completion_rate
            FROM item_features
            WHERE item_id = ANY($1)
            "#,
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let mut result = HashMap::new();

        for row in rows {
            let features = if let Some(embedding) = row.embedding {
                // Use pre-computed embedding
                embedding
            } else {
                // Build features from components
                let mut features = Vec::new();

                if let Some(tfidf) = row.tfidf_vector {
                    if let Ok(vec) = serde_json::from_value::<Vec<f32>>(tfidf) {
                        features.extend(vec);
                    }
                }

                // Normalize numeric features
                features.push(row.view_count.unwrap_or(0) as f32 / 100000.0);
                features.push(row.trending_score.unwrap_or(0.0));
                features.push(row.completion_rate.unwrap_or(0.0));

                features
            };

            result.insert(row.item_id, Self::pad_or_truncate(features, feature_dim));
        }

        // Fill missing items with zeros (cold start)
        for &item_id in item_ids {
            result.entry(item_id).or_insert_with(|| vec![0.0; feature_dim]);
        }

        Ok(result)
    }

    /// Pad or truncate feature vector to exact dimension
    fn pad_or_truncate(mut features: Vec<f32>, target_dim: usize) -> Vec<f32> {
        if features.len() < target_dim {
            features.resize(target_dim, 0.0);
        } else if features.len() > target_dim {
            features.truncate(target_dim);
        }
        features
    }
}
