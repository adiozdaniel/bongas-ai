use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmStatistics {
    pub total_reward: f64,
    pub pulls: u64,
    pub mean_reward: f64,
    pub variance: f64,
}

impl ArmStatistics {
    pub fn new() -> Self {
        Self {
            total_reward: 0.0,
            pulls: 0,
            mean_reward: 0.0,
            variance: 0.0,
        }
    }

    pub fn update(&mut self, reward: f64) {
        self.total_reward += reward;
        self.pulls += 1;

        let old_mean = self.mean_reward;
        self.mean_reward = self.total_reward / self.pulls as f64;

        // Welford's online algorithm for variance
        if self.pulls > 1 {
            let delta = reward - old_mean;
            let delta2 = reward - self.mean_reward;
            self.variance += delta * delta2;
        }
    }

}

pub struct UCB1 {
    arms: HashMap<String, ArmStatistics>,
    total_pulls: u64,
    exploration_constant: f64,
}

impl UCB1 {
    pub fn new(arm_names: Vec<String>, exploration_constant: f64) -> Self {
        let mut arms = HashMap::new();
        for name in arm_names {
            arms.insert(name, ArmStatistics::new());
        }

        Self {
            arms,
            total_pulls: 0,
            exploration_constant,
        }
    }

    /// Select arm using UCB1 algorithm
    pub fn select_arm(&self) -> Result<String> {
        
        // Pull each arm once initially
        for (name, stats) in &self.arms {
            if stats.pulls == 0 {
                
                // Track bandit selection metrics
                
                return Ok(name.clone());
            }
        }

        let mut best_arm = String::new();
        let mut best_ucb = f64::NEG_INFINITY;

        for (name, stats) in &self.arms {
            let ucb = stats.mean_reward
                + self.exploration_constant
                    * ((2.0 * (self.total_pulls as f64).ln()) / stats.pulls as f64).sqrt();

            if ucb > best_ucb {
                best_ucb = ucb;
                best_arm = name.clone();
            }
        }
        
        // Track bandit selection metrics

        Ok(best_arm)
    }

    pub fn update(&mut self, arm_name: &str, reward: f64) -> Result<()> {
        
        let stats = self.arms
            .get_mut(arm_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown arm: {}", arm_name))?;

        stats.update(reward);
        self.total_pulls += 1;
        
        // Track bandit update metrics

        Ok(())
    }

    pub fn get_stats(&self) -> &HashMap<String, ArmStatistics> {
        &self.arms
    }
}
