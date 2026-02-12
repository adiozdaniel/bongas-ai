use anyhow::Result;
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LinUCBArm {
    pub A: DMatrix<f64>,  // d×d design matrix
    pub b: DVector<f64>,  // d×1 reward vector
    pub d: usize,         // Feature dimension
}

impl LinUCBArm {
    pub fn new(d: usize) -> Self {
        Self {
            A: DMatrix::identity(d, d),
            b: DVector::zeros(d),
            d,
        }
    }

    pub fn update(&mut self, context: &DVector<f64>, reward: f64) {
        // A = A + x·x^T
        self.A += context * context.transpose();

        // b = b + r·x
        self.b += reward * context;
    }

    pub fn predict(&self, context: &DVector<f64>, alpha: f64) -> f64 {
        // θ = A^(-1)·b
        let A_inv = self.A.clone().try_inverse().unwrap_or_else(|| {
            // Fallback: use pseudo-inverse
            DMatrix::identity(self.d, self.d)
        });

        let theta = &A_inv * &self.b;

        // Upper confidence bound: θ^T·x + α·sqrt(x^T·A^(-1)·x)
        let prediction = theta.dot(context);
        let uncertainty = alpha * (context.transpose() * &A_inv * context)[(0, 0)].sqrt();

        prediction + uncertainty
    }
}

pub struct LinUCB {
    pub arms: HashMap<String, LinUCBArm>,
    pub alpha: f64,  // Exploration parameter
    pub feature_dim: usize,
}

impl LinUCB {
    pub fn new(arm_names: Vec<String>, feature_dim: usize, alpha: f64) -> Self {
        let mut arms = HashMap::new();
        for name in arm_names {
            arms.insert(name, LinUCBArm::new(feature_dim));
        }

        Self {
            arms,
            alpha,
            feature_dim,
        }
    }

    /// Select arm using default context (no features required)
    pub fn select_arm(&self) -> Result<String> {
        // Use uniform context vector for selection
        let context_vector = DVector::from_vec(vec![1.0; self.feature_dim]);

        let mut best_arm = String::new();
        let mut best_ucb = f64::NEG_INFINITY;

        for (name, arm) in &self.arms {
            let ucb = arm.predict(&context_vector, self.alpha);

            if ucb > best_ucb {
                best_ucb = ucb;
                best_arm = name.clone();
            }
        }

        Ok(best_arm)
    }

    /// Update arm with reward (simplified, no context)
    pub fn update(&mut self, arm_name: &str, reward: f64) -> Result<()> {
        // For simple usage, use a default context vector
        let context_vector = DVector::from_vec(vec![1.0; self.feature_dim]);

        let arm = self.arms
            .get_mut(arm_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown arm: {}", arm_name))?;

        arm.update(&context_vector, reward);

        Ok(())
    }

    /// Update arm with context and reward (full version)
    pub fn update_with_context(
        &mut self,
        arm_name: &str,
        context: &ContextFeatures,
        reward: f64,
    ) -> Result<()> {
        let context_vector = context.to_vector(self.feature_dim)?;

        let arm = self.arms
            .get_mut(arm_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown arm: {}", arm_name))?;

        arm.update(&context_vector, reward);

        Ok(())
    }

    /// Get statistics for all arms
    pub fn get_stats(&self) -> serde_json::Value {
        use serde_json::json;
        
        let arms_stats: serde_json::Map<String, serde_json::Value> = self.arms
            .iter()
            .map(|(name, arm)| {
                (name.clone(), json!({
                    "A_determinant": arm.A.determinant(),
                    "d": arm.d
                }))
            })
            .collect();
        
        json!({
            "arms": arms_stats,
            "alpha": self.alpha,
            "feature_dim": self.feature_dim
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFeatures {
    pub user_id: i32,
    pub time_of_day: f64,       // 0-23
    pub day_of_week: f64,       // 0-6
    pub user_tenure_days: f64,
    pub recent_watch_count: f64,
    pub genre_preferences: Vec<f64>,  // One-hot or embedding
}

impl ContextFeatures {
    /// Convert to feature vector
    pub fn to_vector(&self, expected_dim: usize) -> Result<DVector<f64>> {
        let mut features = vec![
            1.0,  // Bias term
            self.time_of_day / 23.0,  // Normalize
            self.day_of_week / 6.0,
            (self.user_tenure_days / 365.0).min(1.0),
            (self.recent_watch_count / 100.0).min(1.0),
        ];

        features.extend_from_slice(&self.genre_preferences);

        // Pad or truncate to expected dimension
        features.resize(expected_dim, 0.0);

        Ok(DVector::from_vec(features))
    }
}