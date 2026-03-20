//! Native Rust ML Training Service using Candle.
//! Implements on-premise learning (Sovereign Training) for Student Heads.

use candle_core::{Result, Tensor, Device};
use candle_nn::{Optimizer, VarMap};
use tracing::{info, debug};

use crate::ml::training::candle::architectures::ranking::StudentRankingHead;
use crate::ml::training::candle::types::models::{TrainingSample, TrainingBatch};

/// The training service for the Student Ranking Head.
pub struct RankingTrainer {
    device: Device,
}

impl RankingTrainer {
    pub fn new() -> Self {
        Self {
            device: Device::Cpu, // Prioritize CPU for sovereign VPC stability
        }
    }
}

impl Default for RankingTrainer {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingTrainer {
    /// Run a single training epoch on the provided batch.
    pub async fn train_batch(
        &self,
        model: &StudentRankingHead,
        varmap: &mut VarMap,
        batch: TrainingBatch,
        learning_rate: f64,
    ) -> Result<f32> {
        let batch_size = batch.samples.len();
        if batch_size == 0 {
            return Ok(0.0);
        }

        let dna_flat: Vec<f32> = batch.samples.iter().flat_map(|s| s.dna_vector.clone()).collect();
        let targets: Vec<f32> = batch.samples.iter().map(|s| s.target).collect();

        let dna_tensor = Tensor::from_vec(dna_flat, (batch_size, 512), &self.device)?;
        let target_tensor = Tensor::from_vec(targets, (batch_size, 1), &self.device)?;

        // 1. Forward Pass
        let predictions = model.forward(&dna_tensor)?;

        // 2. Calculate Loss (Sovereign Mean Squared Error for Ranking)
        let loss = predictions.broadcast_sub(&target_tensor)?
            .sqr()?
            .mean_all()?;
        
        let loss_val = loss.to_vec0::<f32>()?;

        // 3. Backward Pass (Gradients)
        let mut opt = candle_nn::AdamW::new(
            varmap.all_vars(),
            candle_nn::ParamsAdamW {
                lr: learning_rate,
                ..Default::default()
            },
        )?;
        
        opt.backward_step(&loss)?;

        debug!(loss = %loss_val, "RankingTrainer: Batch completed");
        Ok(loss_val)
    }

    /// Perform a full epoch over a dataset.
    pub async fn run_epoch(
        &self,
        model: &StudentRankingHead,
        varmap: &mut VarMap,
        dataset: Vec<TrainingSample>,
        batch_size: usize,
    ) -> Result<f32> {
        let mut total_loss = 0.0;
        let mut count = 0;

        for chunk in dataset.chunks(batch_size) {
            let batch = TrainingBatch {
                samples: chunk.to_vec(),
            };

            let loss = self.train_batch(model, varmap, batch, 0.001).await?;
            total_loss += loss;
            count += 1;
        }

        if count == 0 {
            return Ok(0.0);
        }

        let avg_loss = total_loss / (count as f32);
        info!(avg_loss = %avg_loss, "RankingTrainer: Epoch completed successfully");
        Ok(avg_loss)
    }
}
