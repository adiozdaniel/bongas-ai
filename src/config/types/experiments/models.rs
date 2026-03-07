//! Configuration for the experimentation and A/B testing system.

use crate::experiments::models::AssignmentMethod;
use serde::Deserialize;

/// Configuration for experiments and multi-armed bandits.
#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentsConfig {
    /// Whether the experimentation system is enabled.
    /// If false, all experiment endpoints return 404 and no traffic is branched.
    #[serde(default)]
    pub enabled: bool,
    
    /// Default assignment method if not specified in the experiment.
    #[serde(default = "default_assignment_method")]
    pub assignment_method: AssignmentMethod,
}

fn default_assignment_method() -> AssignmentMethod {
    AssignmentMethod::Random
}

impl Default for ExperimentsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            assignment_method: AssignmentMethod::Random,
        }
    }
}
