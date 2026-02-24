//! Configuration for the experimentation and A/B testing system.

/// Configuration for experiments and multi-armed bandits.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExperimentsConfig {
    /// Whether the experimentation system is enabled.
    /// If false, all experiment endpoints return 404 and no traffic is branched.
    pub enabled: bool,
    
    /// Default assignment method if not specified in the experiment.
    pub assignment_method: String,
}

impl Default for ExperimentsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            assignment_method: "random".to_string(),
        }
    }
}
