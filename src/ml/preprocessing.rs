use anyhow::Result;
use ndarray::{Array1, Array2};

pub struct FeaturePreprocessor {
    user_feature_dim: usize,
    item_feature_dim: usize,
}

impl FeaturePreprocessor {
    pub fn new(user_feature_dim: usize, item_feature_dim: usize) -> Self {
        Self {
            user_feature_dim,
            item_feature_dim,
        }
    }

    /// Prepare user features for ONNX inference
    pub fn prepare_user_features(&self, features: Vec<f32>) -> Result<Array1<f32>> {
        if features.len() != self.user_feature_dim {
            return Err(anyhow::anyhow!(
                "User feature dimension mismatch: expected {}, got {}",
                self.user_feature_dim,
                features.len()
            ));
        }

        let array = Array1::from_vec(features);
        Ok(self.normalize(array))
    }

    /// Prepare item features for ONNX inference
    pub fn prepare_item_features(&self, features: Vec<f32>) -> Result<Array1<f32>> {
        if features.len() != self.item_feature_dim {
            return Err(anyhow::anyhow!(
                "Item feature dimension mismatch: expected {}, got {}",
                self.item_feature_dim,
                features.len()
            ));
        }

        let array = Array1::from_vec(features);
        Ok(self.normalize(array))
    }

    /// Normalize features (z-score normalization)
    fn normalize(&self, features: Array1<f32>) -> Array1<f32> {
        let mean = features.mean().unwrap_or(0.0);
        let std_dev = features.std(0.0);

        if std_dev > 1e-6 {
            (features - mean) / std_dev
        } else {
            features
        }
    }

    /// Batch prepare user features
    pub fn prepare_user_features_batch(&self, features: Vec<Vec<f32>>) -> Result<Array2<f32>> {
        let batch_size = features.len();
        let mut array = Array2::<f32>::zeros((batch_size, self.user_feature_dim));

        for (i, feat) in features.iter().enumerate() {
            let normalized = self.prepare_user_features(feat.clone())?;
            for (j, &val) in normalized.iter().enumerate() {
                array[[i, j]] = val;
            }
        }

        Ok(array)
    }

    /// Batch prepare item features
    pub fn prepare_item_features_batch(&self, features: Vec<Vec<f32>>) -> Result<Array2<f32>> {
        let batch_size = features.len();
        let mut array = Array2::<f32>::zeros((batch_size, self.item_feature_dim));

        for (i, feat) in features.iter().enumerate() {
            let normalized = self.prepare_item_features(feat.clone())?;
            for (j, &val) in normalized.iter().enumerate() {
                array[[i, j]] = val;
            }
        }

        Ok(array)
    }

    /// Get configured dimensions
    pub fn user_feature_dim(&self) -> usize {
        self.user_feature_dim
    }

    pub fn item_feature_dim(&self) -> usize {
        self.item_feature_dim
    }
}

/// Feature builder for constructing feature vectors from raw data
pub struct FeatureBuilder {
    features: Vec<f32>,
}

impl FeatureBuilder {
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
        }
    }

    pub fn add_scalar(&mut self, value: f32) -> &mut Self {
        self.features.push(value);
        self
    }

    pub fn add_vector(&mut self, values: &[f32]) -> &mut Self {
        self.features.extend_from_slice(values);
        self
    }

    pub fn add_categorical(&mut self, value: usize, num_categories: usize) -> &mut Self {
        // One-hot encoding
        for i in 0..num_categories {
            self.features.push(if i == value { 1.0 } else { 0.0 });
        }
        self
    }

    pub fn build(self) -> Vec<f32> {
        self.features
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

impl Default for FeatureBuilder {
    fn default() -> Self {
        Self::new()
    }
}
