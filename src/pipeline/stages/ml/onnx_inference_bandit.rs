use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::{info, warn, debug};

#[derive(Deserialize)]
struct Params {
    /// Model name for value prediction (optional - uses heuristics if not specified)
    #[serde(default)]
    model_name: Option<String>,
    /// Bandit algorithm: "epsilon_greedy", "thompson_sampling", "ucb", "linucb"
    #[serde(default = "default_algorithm")]
    algorithm: String,
    /// Exploration rate (epsilon for epsilon-greedy, alpha for UCB)
    #[serde(default = "default_explore_rate")]
    explore_rate: f32,
    /// Temperature for softmax-based exploration
    #[serde(default = "default_temperature")]
    temperature: f32,
    /// User feature dimension for contextual bandits
    #[serde(default = "default_user_dim")]
    user_feature_dim: usize,
    /// Item feature dimension for contextual bandits
    #[serde(default = "default_item_dim")]
    item_feature_dim: usize,
    /// Experiment/campaign ID for tracking
    #[serde(default)]
    experiment_id: Option<String>,
}

fn default_algorithm() -> String { "epsilon_greedy".to_string() }
fn default_explore_rate() -> f32 { 0.1 }
fn default_temperature() -> f32 { 1.0 }
fn default_user_dim() -> usize { 64 }
fn default_item_dim() -> usize { 32 }

pub struct ONNXInferenceBanditStage;

#[async_trait]
impl PipelineStage for ONNXInferenceBanditStage {
    fn name(&self) -> &str {
        "onnx_inference_bandit"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse onnx_inference_bandit params")?;

        let user_id = context.user_id
            .ok_or_else(|| anyhow::anyhow!("user_id required for bandit inference"))?;

        if input.is_empty() {
            debug!(
                request_id = %context.request_id,
                "Bandit inference skipped: no input items"
            );
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            algorithm = %params.algorithm,
            explore_rate = params.explore_rate,
            input_count = input.len(),
            user_id = user_id,
            "Running ONNX bandit inference"
        );

        let start = std::time::Instant::now();

        // Get base scores from ONNX model if specified
        let base_scores: Vec<f32> = if let Some(model_name) = &params.model_name {
            self.get_onnx_scores(context, model_name, user_id, &input, &params).await?
        } else {
            // Use existing scores from input items
            input.iter().map(|item| item.score).collect()
        };

        // Apply bandit algorithm for exploration/exploitation
        let mut results: Vec<ScoredItem> = input
            .into_iter()
            .zip(base_scores.iter())
            .enumerate()
            .map(|(idx, (mut item, &base_score))| {
                let exploration_bonus = self.compute_exploration_bonus(
                    &params.algorithm,
                    params.explore_rate,
                    params.temperature,
                    base_score,
                    idx,
                    item.item_id,
                );

                item.score = base_score + exploration_bonus;
                item.metadata["bandit_algorithm"] = json!(params.algorithm);
                item.metadata["explore_rate"] = json!(params.explore_rate);
                item.metadata["base_score"] = json!(base_score);
                item.metadata["exploration_bonus"] = json!(exploration_bonus);
                item.metadata["inference_engine"] = json!("onnx_bandit");

                if let Some(exp_id) = &params.experiment_id {
                    item.metadata["experiment_id"] = json!(exp_id);
                }

                item
            })
            .collect();

        // Sort by final score
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        let inference_time = start.elapsed();

        info!(
            request_id = %context.request_id,
            algorithm = %params.algorithm,
            output_count = results.len(),
            inference_ms = inference_time.as_millis() as u64,
            "Bandit inference complete"
        );

        Ok(results)
    }
}

impl ONNXInferenceBanditStage {
    /// Get scores from ONNX model
    async fn get_onnx_scores(
        &self,
        context: &ExecutionContext,
        model_name: &str,
        user_id: i32,
        items: &[ScoredItem],
        params: &Params,
    ) -> Result<Vec<f32>> {
        // Try to load model
        let model = match context.model_loader.get_model(model_name).await {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    model_name = model_name,
                    error = %e,
                    "Failed to load ONNX model for bandit, using input scores"
                );
                return Ok(items.iter().map(|item| item.score).collect());
            }
        };

        // Get user features
        let user_features = self
            .get_user_features(context, user_id, params.user_feature_dim)
            .await?;

        // Get item features
        let item_ids: Vec<i32> = items.iter().map(|item| item.item_id).collect();
        let item_features_map = self
            .get_item_features(context, &item_ids, params.item_feature_dim)
            .await?;

        // Prepare batch inputs
        let mut user_batch: Vec<Vec<f32>> = Vec::new();
        let mut item_batch: Vec<Vec<f32>> = Vec::new();

        for item in items {
            let item_feats = item_features_map
                .get(&item.item_id)
                .cloned()
                .unwrap_or_else(|| vec![0.0; params.item_feature_dim]);

            user_batch.push(user_features.clone());
            item_batch.push(item_feats);
        }

        // Run inference
        if user_batch.is_empty() {
            return Ok(items.iter().map(|item| item.score).collect());
        }

        let mut engine = model.write().await;
        let scores = engine
            .predict_batch(user_batch, item_batch)
            .context("ONNX bandit inference failed")?;

        Ok(scores)
    }

    /// Compute exploration bonus based on algorithm
    fn compute_exploration_bonus(
        &self,
        algorithm: &str,
        explore_rate: f32,
        temperature: f32,
        base_score: f32,
        index: usize,
        item_id: i32,
    ) -> f32 {
        match algorithm {
            "epsilon_greedy" => {
                // With probability epsilon, add random exploration bonus
                let rand = pseudo_random(item_id, index);
                if rand < explore_rate {
                    // Exploration: add random bonus
                    (pseudo_random(item_id + 1, index) - 0.5) * 2.0
                } else {
                    // Exploitation: no bonus
                    0.0
                }
            }
            "thompson_sampling" => {
                // Sample from posterior (approximated with noise)
                let noise = (pseudo_random(item_id, index) - 0.5) * 2.0;
                noise * explore_rate * temperature
            }
            "ucb" | "ucb1" => {
                // Upper Confidence Bound
                // UCB = mean + alpha * sqrt(ln(t) / n)
                // Simplified: add confidence bonus based on uncertainty
                let uncertainty = 1.0 / (base_score.abs() + 0.1).sqrt();
                explore_rate * uncertainty
            }
            "linucb" => {
                // Linear UCB (contextual)
                // Exploration bonus based on feature uncertainty
                let feature_uncertainty = pseudo_random(item_id, index);
                explore_rate * feature_uncertainty.sqrt()
            }
            "softmax" | "boltzmann" => {
                // Softmax/Boltzmann exploration
                // Add temperature-scaled noise
                let noise = (pseudo_random(item_id, index) - 0.5) * 2.0;
                noise / temperature
            }
            _ => 0.0,
        }
    }

    /// Get user features from database
    async fn get_user_features(
        &self,
        context: &ExecutionContext,
        user_id: i32,
        feature_dim: usize,
    ) -> Result<Vec<f32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            embedding: Option<Vec<f32>>,
        }

        let row: Option<Row> = sqlx::query_as(
            "SELECT embedding FROM user_features WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        let features = match row {
            Some(Row { embedding: Some(emb) }) => emb,
            _ => vec![0.0; feature_dim],
        };

        Ok(pad_or_truncate(features, feature_dim))
    }

    /// Get item features from database
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
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT item_id, embedding FROM item_features WHERE item_id = ANY($1)",
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let mut result = HashMap::new();

        for row in rows {
            let features = row.embedding.unwrap_or_else(|| vec![0.0; feature_dim]);
            result.insert(row.item_id, pad_or_truncate(features, feature_dim));
        }

        // Fill missing
        for &item_id in item_ids {
            result.entry(item_id).or_insert_with(|| vec![0.0; feature_dim]);
        }

        Ok(result)
    }
}

/// Deterministic pseudo-random score based on item_id and index
fn pseudo_random(item_id: i32, index: usize) -> f32 {
    let combined = (item_id as u64).wrapping_mul(2654435761).wrapping_add(index as u64);
    let hash = combined.wrapping_mul(1103515245).wrapping_add(12345);
    ((hash >> 16) % 1000) as f32 / 1000.0
}

/// Pad or truncate vector to exact dimension
fn pad_or_truncate(mut features: Vec<f32>, target_dim: usize) -> Vec<f32> {
    if features.len() < target_dim {
        features.resize(target_dim, 0.0);
    } else if features.len() > target_dim {
        features.truncate(target_dim);
    }
    features
}
