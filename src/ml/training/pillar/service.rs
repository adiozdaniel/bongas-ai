//! 🏗️ THE BACKSTAGE: Training & Learning Pillar (Symphony 3.0)
//! 
//! Orchestrates the background learning lifecycle, online feedback, and 
//! native Rust training logic (Candle).

use std::sync::Arc;
use crate::ml::training::online::service::OnlineLearningManager;
use crate::ml::training::pillar::worker::SovereignTrainingPillar;
use crate::ml::training::pillar::state::TrainingState;

/// High-level orchestrator for the Sovereign Training domain.
pub struct TrainingPillar {
    /// Ingestion of real-time user feedback.
    pub online: Arc<OnlineLearningManager>,
    /// Background worker for on-premise learning.
    pub sovereign: Arc<SovereignTrainingPillar>,
    /// Shared in-memory and disk state for the training process.
    pub state: Arc<TrainingState>,
}

impl TrainingPillar {
    pub fn new(
        online: Arc<OnlineLearningManager>,
        sovereign: Arc<SovereignTrainingPillar>,
        state: Arc<TrainingState>,
    ) -> Self {
        Self {
            online,
            sovereign,
            state,
        }
    }

    /// Start the background training orchestration.
    pub async fn start(&self) {
        // Start System Health Polling (M21.1)
        let health = self.sovereign.circuit.health.clone();
        tokio::spawn(async move {
            health.start_polling(std::time::Duration::from_secs(1)).await;
        });

        // Start Sovereign Training Pillar (M21.5)
        let sovereign = self.sovereign.clone();
        tokio::spawn(async move {
            sovereign.start().await;
        });
    }
}
