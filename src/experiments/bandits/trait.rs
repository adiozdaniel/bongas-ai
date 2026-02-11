use serde::{Deserialize, Serialize};

/// Common trait for all bandit algorithms
#[async_trait::async_trait]
pub trait BanditAlgorithm: Send + Sync + 'static {
    /// Select the best variant based on current knowledge
    fn select_variant(&self, variant_ids: &[String]) -> Option<String>;

    /// Update the algorithm with trial results
    fn update(&mut self, variant_id: &str, success: bool);

    /// Get algorithm-specific statistics
    fn get_stats(&self) -> serde_json::Value;
}

/// Bandit algorithm types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BanditType {
    ThompsonSampling,
    UCB1,
    UCBTuned,
    EpsilonGreedy,
    LinUCB,
}

/// Configuration for bandit algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanditConfig {
    pub algorithm: BanditType,
    pub epsilon: Option<f64>,  // For epsilon-greedy
    pub exploration_weight: Option<f64>,  // For UCB variants
    pub context_dimensions: Option<usize>,  // For LinUCB
}