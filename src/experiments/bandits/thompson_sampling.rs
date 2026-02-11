use anyhow::Result;
use rand::Rng;
use rand_distr::{Beta, Distribution};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaDistribution {
    pub alpha: f64,  // Successes
    pub beta: f64,   // Failures
}

impl BetaDistribution {
    pub fn new() -> Self {
        Self {
            alpha: 1.0,  // Uniform prior
            beta: 1.0,
        }
    }

    pub fn sample(&self, rng: &mut impl Rng) -> f64 {
        let dist = Beta::new(self.alpha, self.beta).unwrap();
        dist.sample(rng)
    }

    pub fn update_success(&mut self) {
        self.alpha += 1.0;
    }

    pub fn update_failure(&mut self) {
        self.beta += 1.0;
    }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }
}

pub struct ThompsonSampling {
    arms: HashMap<String, BetaDistribution>,
    total_pulls: u64,
}

impl ThompsonSampling {
    pub fn new(arm_names: Vec<String>) -> Self {
        let mut arms = HashMap::new();
        for name in arm_names {
            arms.insert(name, BetaDistribution::new());
        }

        Self {
            arms,
            total_pulls: 0,
        }
    }

    /// Select an arm based on Thompson Sampling
    pub fn select_arm(&self) -> Result<String> {
        let mut rng = rand::thread_rng();
        let mut best_arm = String::new();
        let mut best_sample = f64::NEG_INFINITY;

        for (arm_name, dist) in &self.arms {
            let sample = dist.sample(&mut rng);
            if sample > best_sample {
                best_sample = sample;
                best_arm = arm_name.clone();
            }
        }

        Ok(best_arm)
    }

    /// Update arm with reward (1.0 = success, 0.0 = failure)
    pub fn update(&mut self, arm_name: &str, reward: f64) -> Result<()> {
        let dist = self.arms
            .get_mut(arm_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown arm: {}", arm_name))?;

        if reward > 0.5 {
            dist.update_success();
        } else {
            dist.update_failure();
        }

        self.total_pulls += 1;

        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> HashMap<String, ArmStats> {
        self.arms
            .iter()
            .map(|(name, dist)| {
                (
                    name.clone(),
                    ArmStats {
                        mean: dist.mean(),
                        alpha: dist.alpha,
                        beta: dist.beta,
                        pulls: (dist.alpha + dist.beta - 2.0) as u64,
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmStats {
    pub mean: f64,
    pub alpha: f64,
    pub beta: f64,
    pub pulls: u64,
}