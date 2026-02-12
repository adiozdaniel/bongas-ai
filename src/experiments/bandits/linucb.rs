use anyhow::Result;
use nalgebra::{DMatrix, DVector};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LinUCBArm {
    pub a: DMatrix<f64>,  // d×d design matrix
    pub b_vec: DVector<f64>,  // d×1 reward vector
    pub dim: usize,         // Feature dimension
}

impl LinUCBArm {
    pub fn new(dim: usize) -> Self {
        Self {
            a: DMatrix::identity(dim, dim),
            b_vec: DVector::zeros(dim),
            dim,
        }
    }

    pub fn update(&mut self, context: &DVector<f64>, reward: f64) {
        // A = A + x·x^T
        self.a += context * context.transpose();

        // b = b + r·x
        self.b_vec += reward * context;
    }

    pub fn predict(&self, context: &DVector<f64>, alpha: f64) -> f64 {
        // θ = A^(-1)·b
        let a_inv = self.a.clone().try_inverse().unwrap_or_else(|| {
            // Fallback: use pseudo-inverse
            DMatrix::identity(self.dim, self.dim)
        });

        let theta = &a_inv * &self.b_vec;

        // Upper confidence bound: θ^T·x + α·sqrt(x^T·A^(-1)·x)
        let prediction = theta.dot(context);
        let uncertainty = alpha * (context.transpose() * &a_inv * context)[(0, 0)].sqrt();

        prediction + uncertainty
    }
}

pub struct LinUCB {
    pub arms: HashMap<String, LinUCBArm>,
    pub alpha: f64,  // Exploration parameter
    pub feature_dim: usize,
}

impl LinUCB {
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

}