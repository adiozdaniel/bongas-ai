use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

use crate::ml::model_loader::ModelLoader;
use crate::ml::preprocessing::FeaturePreprocessor;
use crate::ml::postprocessing::ScorePostprocessor;

pub struct InferenceService {
    model_loader: Arc<ModelLoader>,
    preprocessor: FeaturePreprocessor,
    postprocessor: ScorePostprocessor,
}

impl InferenceService {
    pub fn new(
        model_loader: Arc<ModelLoader>,
        user_feature_dim: usize,
        item_feature_dim: usize,
    ) -> Self {
        Self {
            model_loader,
            preprocessor: FeaturePreprocessor::new(user_feature_dim, item_feature_dim),
            postprocessor: ScorePostprocessor::new(),
        }
    }

    /// Recommend items for a user using ONNX model
    pub async fn recommend(
        &self,
        model_name: &str,
        version: &str,
        user_features: Vec<f32>,
        candidate_items: Vec<(i32, Vec<f32>)>, // (item_id, features)
        top_k: usize,
    ) -> Result<Vec<(i32, f32)>> {
        debug!(
            model = model_name,
            version = version,
            candidates = candidate_items.len(),
            "Running inference"
        );

        // Get model
        let engine = self.model_loader.get_model_version(model_name, version).await?;

        // Prepare user features (replicate for batch)
        let batch_size = candidate_items.len();
        let user_batch: Vec<Vec<f32>> = vec![user_features; batch_size];

        // Prepare item features
        let item_batch: Vec<Vec<f32>> = candidate_items.iter().map(|(_, f)| f.clone()).collect();

        // Preprocess
        let user_array = self.preprocessor.prepare_user_features_batch(user_batch)?;
        let item_array = self.preprocessor.prepare_item_features_batch(item_batch)?;

        // Run ONNX inference
        let scores = engine.write().await.predict_two_tower(user_array, item_array)?;

        // Postprocess
        let normalized_scores = self.postprocessor.normalize_scores(&scores);
        let top_indices = self.postprocessor.top_k(&normalized_scores, top_k);

        // Map back to item IDs
        let recommendations: Vec<(i32, f32)> = top_indices
            .into_iter()
            .map(|(idx, score)| (candidate_items[idx].0, score))
            .collect();

        info!(
            model = model_name,
            version = version,
            result_count = recommendations.len(),
            "Inference complete"
        );

        Ok(recommendations)
    }

    /// Recommend using latest deployed model version
    pub async fn recommend_latest(
        &self,
        model_name: &str,
        user_features: Vec<f32>,
        candidate_items: Vec<(i32, Vec<f32>)>,
        top_k: usize,
    ) -> Result<Vec<(i32, f32)>> {
        debug!(
            model = model_name,
            candidates = candidate_items.len(),
            "Running inference with latest model"
        );

        let engine = self.model_loader.get_latest_model(model_name).await?;

        let batch_size = candidate_items.len();
        let user_batch: Vec<Vec<f32>> = vec![user_features; batch_size];
        let item_batch: Vec<Vec<f32>> = candidate_items.iter().map(|(_, f)| f.clone()).collect();

        let user_array = self.preprocessor.prepare_user_features_batch(user_batch)?;
        let item_array = self.preprocessor.prepare_item_features_batch(item_batch)?;

        let scores = engine.write().await.predict_two_tower(user_array, item_array)?;
        let normalized_scores = self.postprocessor.normalize_scores(&scores);
        let top_indices = self.postprocessor.top_k(&normalized_scores, top_k);

        let recommendations: Vec<(i32, f32)> = top_indices
            .into_iter()
            .map(|(idx, score)| (candidate_items[idx].0, score))
            .collect();

        Ok(recommendations)
    }
}
