//! Models for the experimentation and A/B testing system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An active experiment (A/B test or Multi-Armed Bandit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub variants: Vec<Variant>,
    pub assignment_method: AssignmentMethod,
    pub status: ExperimentStatus,
}

/// A specific variant within an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub weight: f32, // 0.0 to 1.0
    /// Configuration overrides for this variant (e.g., {"model_name": "grok_v2"})
    pub overrides: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentMethod {
    Random,
    Hash, // Consistent hashing based on user_id
    ThompsonSampling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Draft,
    Running,
    Completed,
    Archived,
}

/// The result of a user assignment to an experiment.
#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub experiment_id: i32,
    pub experiment_slug: String,
    pub variant_id: String,
    pub overrides: HashMap<String, serde_json::Value>,
}
