//! Native Rust Language Model Trainer using Candle.
//! Implements on-premise fine-tuning (Continuous Forge) for the Swahili Brain.

use candle_core::{Result, Tensor};
use candle_nn::{Optimizer, VarMap};
use tracing::debug;

use crate::ml::training::candle::architectures::language::StudentLanguageHead;

pub struct LanguageBatch {
    pub input_ids: Tensor,
    pub labels: Tensor,
}

/// The training service for the Student Language Head.
pub struct LanguageTrainer {}

impl LanguageTrainer {
    pub fn new() -> Self {
        Self {}
    }

    /// Run a single training batch.
    pub async fn train_batch(
        &self,
        model: &StudentLanguageHead,
        varmap: &mut VarMap,
        batch: LanguageBatch,
        learning_rate: f64,
    ) -> Result<f32> {
        // 1. Forward Pass
        let logits = model.forward(&batch.input_ids)?;
        
        // 2. Calculate Cross-Entropy Loss
        // logits: [batch, seq_len, vocab_size]
        // labels: [batch, seq_len]
        let (batch_size, seq_len, vocab_size) = logits.dims3()?;
        let logits_flat = logits.reshape((batch_size * seq_len, vocab_size))?;
        let labels_flat = batch.labels.reshape((batch_size * seq_len,))?;
        
        // Use Candle's log_softmax + gather for cross entropy
        let log_probs = candle_nn::ops::log_softmax(&logits_flat, candle_core::D::Minus1)?;
        let loss = candle_nn::loss::nll(&log_probs, &labels_flat)?;
        
        let loss_val = loss.to_vec0::<f32>()?;

        // 3. Backward Pass (Gradients)
        // In LoRA training, we would ideally only optimize the lora_a and lora_b parameters.
        // For this implementation, we assume the varmap contains only trainable parameters.
        let mut opt = candle_nn::AdamW::new(
            varmap.all_vars(),
            candle_nn::ParamsAdamW {
                lr: learning_rate,
                ..Default::default()
            },
        )?;
        
        opt.backward_step(&loss)?;

        debug!(loss = %loss_val, "LanguageTrainer: Batch completed");
        Ok(loss_val)
    }
}

impl Default for LanguageTrainer {
    fn default() -> Self {
        Self::new()
    }
}
