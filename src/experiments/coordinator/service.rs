//! Coordinator for A/B testing and experimentation.
//!
//! Uses consistent hashing for O(1) user assignment and ArcSwap for
//! zero-lock read access in the hot path.

use std::collections::HashMap;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::info;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::experiments::models::{Experiment, Assignment, AssignmentMethod, ExperimentStatus, Variant};
use crate::config::ExperimentsConfig;

pub struct ExperimentCoordinator {
    config: ExperimentsConfig,
    /// Active experiments indexed by scenario slug (one scenario can have multiple experiments)
    active_experiments: ArcSwap<HashMap<String, Vec<Experiment>>>,
}

impl ExperimentCoordinator {
    pub fn new(config: ExperimentsConfig) -> Self {
        Self {
            config,
            active_experiments: ArcSwap::from_pointee(HashMap::new()),
        }
    }

    /// Assign a user to variants for a given scenario.
    /// Returns a list of assignments and a combined map of parameter overrides.
    pub fn assign(
        &self,
        scenario_slug: &str,
        user_id: i32,
    ) -> (Vec<Assignment>, HashMap<String, serde_json::Value>) {
        if !self.config.enabled {
            return (Vec::new(), HashMap::new());
        }

        let experiments_map = self.active_experiments.load();
        let experiments = match experiments_map.get(scenario_slug) {
            Some(exps) => exps,
            None => return (Vec::new(), HashMap::new()),
        };

        let mut assignments = Vec::new();
        let mut combined_overrides = HashMap::new();

        for exp in experiments {
            if exp.status != ExperimentStatus::Running {
                continue;
            }

            if let Some(variant) = self.select_variant(exp, user_id) {
                let assignment = Assignment {
                    experiment_id: exp.id,
                    experiment_slug: exp.slug.clone(),
                    variant_id: variant.id.clone(),
                    overrides: variant.overrides.clone(),
                };

                // Merge overrides
                for (key, value) in &variant.overrides {
                    combined_overrides.insert(key.clone(), value.clone());
                }

                assignments.push(assignment);
            }
        }

        (assignments, combined_overrides)
    }

    fn select_variant<'a>(&self, experiment: &'a Experiment, user_id: i32) -> Option<&'a Variant> {
        if experiment.variants.is_empty() {
            return None;
        }

        match experiment.assignment_method {
            AssignmentMethod::Hash => {
                // Consistent Hashing: hash(user_id + experiment_id) % 100
                let mut hasher = DefaultHasher::new();
                user_id.hash(&mut hasher);
                experiment.id.hash(&mut hasher);
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
                // Random assignment based on variant weights
                let mut hasher = DefaultHasher::new();
                std::time::Instant::now().hash(&mut hasher);
                user_id.hash(&mut hasher);
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
            AssignmentMethod::ThompsonSampling => {
                // ThompsonSampling: In a production environment, this would pull from 
                // the bandit_scores table (Phase 14). For now, we use a weighted random 
                // selection as the probability matching baseline.
                let mut hasher = DefaultHasher::new();
                std::time::Instant::now().hash(&mut hasher);
                user_id.hash(&mut hasher);
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
        }
    }

    /// Refresh the active experiments map (called by background worker)
    pub fn refresh_experiments(&self, new_map: HashMap<String, Vec<Experiment>>) {
        let count: usize = new_map.values().map(|v| v.len()).sum();
        self.active_experiments.store(Arc::new(new_map));
        info!(active_experiments = count, "Experiment Coordinator refreshed");
    }
}
