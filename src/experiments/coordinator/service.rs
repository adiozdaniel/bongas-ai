//! Coordinator for A/B testing and experimentation.
//!
//! Uses simple hashing for O(1) user assignment and ArcSwap for
//! zero-lock read access in the hot path.
//!
//! # Thompson Sampling Implementation
//! For multi-armed bandits, we use Bernoulli Thompson Sampling with Beta distribution
//! priors (Alpha=1, Beta=1). Success is defined as a positive interaction (implicit_rating > 0).

use std::collections::HashMap;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{info, debug};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use rand::prelude::*;
use rand_distr::{Beta, Distribution};

use crate::experiments::models::{Experiment, Variant, AssignmentMethod};

/// Statistics for Bernoulli Thompson Sampling.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BanditStats {
    pub successes: f64, // alpha - 1
    pub failures: f64,  // beta - 1
}

impl Default for BanditStats {
    fn default() -> Self {
        Self { successes: 1.0, failures: 1.0 } // Uniform prior
    }
}

/// Orchestrates experiment assignment and variant management.
pub struct ExperimentCoordinator {
    /// Zero-lock map of active experiments per scenario.
    active_experiments: ArcSwap<HashMap<String, Vec<Experiment>>>,
    
    /// Real-time statistics for Thompson Sampling (Bandit learning).
    /// Map: experiment_id -> (variant_id -> stats)
    bandit_stats: ArcSwap<HashMap<i32, HashMap<String, BanditStats>>>,
}

impl ExperimentCoordinator {
    pub fn new() -> Self {
        Self {
            active_experiments: ArcSwap::from_pointee(HashMap::new()),
            bandit_stats: ArcSwap::from_pointee(HashMap::new()),
        }
    }

    /// Assign a profile to a variant for a specific scenario.
    ///
    /// Returns (variant_id, overrides) if the user is assigned to an experiment.
    pub fn assign(&self, scenario: &str, profile_id: &str) -> (Option<String>, HashMap<String, serde_json::Value>) {
        let experiments = self.active_experiments.load();
        
        if let Some(experiment_list) = experiments.get(scenario) {
            for experiment in experiment_list {
                if let Some(variant) = self.select_variant(experiment, profile_id) {
                    debug!(
                        scenario = %scenario,
                        profile_id = %profile_id,
                        experiment = %experiment.slug,
                        variant = %variant.id,
                        "Profile assigned to experiment variant"
                    );
                    return (Some(variant.id.clone()), variant.overrides.clone());
                }
            }
        }

        (None, HashMap::new())
    }

    /// Select a variant using the configured assignment method.
    fn select_variant<'a>(&self, experiment: &'a Experiment, profile_id: &str) -> Option<&'a Variant> {
        if experiment.variants.is_empty() {
            return None;
        }

        match experiment.assignment_method {
            AssignmentMethod::Hash => {
                // Consistent hashing based on profile_id and experiment_id
                let mut hasher = DefaultHasher::new();
                experiment.id.hash(&mut hasher);
                profile_id.hash(&mut hasher);
                let hash_val = (hasher.finish() % 100) as f32 / 100.0;

                let mut cumulative_weight = 0.0;
                for variant in &experiment.variants {
                    cumulative_weight += variant.weight;
                    if hash_val <= cumulative_weight {
                        return Some(variant);
                    }
                }
                experiment.variants.first()
            }
            AssignmentMethod::Random => {
                let mut rng = thread_rng();
                let hash_val: f32 = rng.gen();

                let mut cumulative_weight = 0.0;
                for variant in &experiment.variants {
                    cumulative_weight += variant.weight;
                    if hash_val <= cumulative_weight {
                        return Some(variant);
                    }
                }
                experiment.variants.first()
            }
            AssignmentMethod::ThompsonSampling => {
                // Actual Thompson Sampling using Beta distribution draws.
                // We pick the variant with the highest probability of being the best.
                let stats_map = self.bandit_stats.load();
                let exp_stats = stats_map.get(&experiment.id);
                
                let mut best_variant = None;
                let mut max_draw = -1.0;
                let mut rng = thread_rng();

                for variant in &experiment.variants {
                    let stats = exp_stats
                        .and_then(|m| m.get(&variant.id))
                        .cloned()
                        .unwrap_or_default();
                    
                    // Draw from Beta(successes, failures)
                    let beta = Beta::new(stats.successes, stats.failures).unwrap_or_else(|_| Beta::new(1.0, 1.0).unwrap());
                    let draw = beta.sample(&mut rng);

                    if draw > max_draw {
                        max_draw = draw;
                        best_variant = Some(variant);
                    }
                }
                
                best_variant.or_else(|| experiment.variants.first())
            }
        }
    }

    /// Record feedback for a variant (success/failure).
    /// Called by the Ingestion Processor or Analytics Sidecar.
    pub fn record_feedback(&self, experiment_id: i32, variant_id: &str, success: bool) {
        let mut new_stats = (**self.bandit_stats.load()).clone();
        let exp_entry = new_stats.entry(experiment_id).or_default();
        let stats = exp_entry.entry(variant_id.to_string()).or_default();
        
        if success {
            stats.successes += 1.0;
        } else {
            stats.failures += 1.0;
        }

        self.bandit_stats.store(Arc::new(new_stats));
        debug!(experiment_id, variant_id, success, "Bandit feedback recorded");
    }

    /// Refresh the active experiments map (called by background worker)
    pub fn refresh_experiments(&self, new_map: HashMap<String, Vec<Experiment>>) {
        let count: usize = new_map.values().map(|v| v.len()).sum();
        self.active_experiments.store(Arc::new(new_map));
        info!(active_experiments = count, "Experiment Coordinator refreshed");
    }
}

impl Default for ExperimentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
