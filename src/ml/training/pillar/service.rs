use std::sync::Arc;
use crate::ml::training::online::service::OnlineLearningManager;
use crate::ml::training::orchestration::service::TrainingOrchestrator;
use crate::ml::training::workers::service::MlWorkerQueue;

/// 🏗️ THE BACKSTAGE: Training & Learning Pillar.
/// 
/// Manages heavy compute tasks, model refinement, and background intelligence.
pub struct TrainingPillar {
    pub online: Arc<OnlineLearningManager>,
    pub orchestration: Arc<TrainingOrchestrator>,
    pub workers: Arc<MlWorkerQueue>,
}

impl TrainingPillar {
    pub fn new(
        online: Arc<OnlineLearningManager>,
        orchestration: Arc<TrainingOrchestrator>,
        workers: Arc<MlWorkerQueue>,
    ) -> Self {
        Self {
            online,
            orchestration,
            workers,
        }
    }
}
