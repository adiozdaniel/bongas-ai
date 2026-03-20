//! Coordination domain for Machine Learning.
//! Orchestrates the Inference and Training pillars.

use std::sync::Arc;
use crate::ml::inference::pillar::service::InferencePillar;
use crate::ml::training::pillar::service::TrainingPillar;
use crate::ml::assets::pillar::service::AssetsPillar;

/// Unified orchestrator for all ML-related activities.
pub struct MlPillar {
    pub inference: Arc<InferencePillar>,
    pub training: Arc<TrainingPillar>,
    pub assets: Arc<AssetsPillar>,
}

impl MlPillar {
    pub fn new(
        inference: Arc<InferencePillar>,
        training: Arc<TrainingPillar>,
        assets: Arc<AssetsPillar>,
    ) -> Self {
        Self {
            inference,
            training,
            assets,
        }
    }
}
