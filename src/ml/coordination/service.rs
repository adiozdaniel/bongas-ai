use std::sync::Arc;
use crate::ml::inference::pillar::service::InferencePillar;
use crate::ml::training::pillar::service::TrainingPillar;
use crate::ml::assets::pillar::service::AssetsPillar;

/// 🧬 THE CORTEX: The central ML coordination handle for BONGAS-AI.
/// 
/// Orchestrates Inference, Training, and ML Assets into a unified Discovery Cortex.
pub struct DiscoveryCortex {
    pub inference: Arc<InferencePillar>,
    pub training: Arc<TrainingPillar>,
    pub assets: Arc<AssetsPillar>,
}

impl DiscoveryCortex {
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
