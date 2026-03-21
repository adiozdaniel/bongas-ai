//! Shared types and data models for the Candle-based training pillar.

use serde::{Deserialize, Serialize};

/// A single training sample consisting of a DNA feature vector and an engagement target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub dna_vector: Vec<f32>,
    pub tribe_vector: Vec<f32>,
    pub target: f32,
}

/// A batch of training samples for the native training loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    pub samples: Vec<TrainingSample>,
}
