//! Sovereign Training Pillar Worker.
//! Orchestrates the background training lifecycle with resource-aware yielding.

use std::sync::Arc;
use tracing::{info, error, debug};
use std::time::Duration;

use crate::ml::training::pillar::circuit::ResourceCircuitBreaker;
use crate::ml::training::pillar::state::TrainingState;
use crate::error::ModelError;

/// Sovereign Training Pillar that orchestrates the background learning lifecycle.
pub struct SovereignTrainingPillar {
    pub(super) circuit: Arc<ResourceCircuitBreaker>,
    pub(super) state: Arc<TrainingState>,
}

impl SovereignTrainingPillar {
    pub fn new(
        circuit: Arc<ResourceCircuitBreaker>,
        state: Arc<TrainingState>,
    ) -> Self {
        Self {
            circuit,
            state,
        }
    }

    /// Background worker to run the training lifecycle.
    pub async fn start(self: Arc<Self>) {
        info!("SovereignTrainingPillar: Initializing background learning loop...");

        loop {
            // 1. Observe: Check if resources are healthy
            if !self.circuit.check_resources().await {
                debug!("SovereignTrainingPillar: Resource saturation detected. Yielding...");
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            // 2. Execute: Run a single training epoch
            info!("SovereignTrainingPillar: Starting training epoch");
            
            let result = self.circuit.call(|| async {
                self.simulate_training_epoch().await;
                Ok::<(), ModelError>(())
            }).await;

            match result {
                Ok(_) => {
                    info!("SovereignTrainingPillar: Epoch completed successfully");
                    self.state.increment_epoch().await;
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
                Err(e) => {
                    error!(error = ?e, "SovereignTrainingPillar: Training epoch failed or was rejected");
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn simulate_training_epoch(&self) {
        for i in 0..10 {
            debug!("Training batch {}/10...", i);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
