//! Native Rust Ranking Head architecture using Candle.
//! Implements the Tribe Conductor for persona-based ranking.

use candle_core::{Result, Tensor};
use candle_nn::{Linear, LayerNorm, Module, VarBuilder};

/// The Local Student Head for Persona-Based Ranking.
/// Learns aggregate affinities between Behavioral Tribes and Content DNA.
pub struct StudentRankingHead {
    ln1: Linear,
    norm1: LayerNorm,
    ln2: Linear,
    ln3: Linear,
}

impl StudentRankingHead {
    /// Create a new ranking head with specified dimensions.
    /// Matches the architecture defined in the 'bongas-ml' trainer.
    pub fn new(vs: VarBuilder) -> Result<Self> {
        let ln1 = candle_nn::linear(64 + 1024, 128, vs.pp("mlp.0"))?;
        let norm1 = candle_nn::layer_norm(128, 1e-5, vs.pp("mlp.1"))?;
        let ln2 = candle_nn::linear(128, 64, vs.pp("mlp.4"))?;
        let ln3 = candle_nn::linear(64, 1, vs.pp("mlp.6"))?;
        
        Ok(Self { ln1, norm1, ln2, ln3 })
    }

    /// Forward pass through the head (Early Fusion).
    pub fn forward(&self, tribe_embedding: &Tensor, item_dna: &Tensor) -> Result<Tensor> {
        let combined = Tensor::cat(&[tribe_embedding, item_dna], 1)?;
        
        let x = self.ln1.forward(&combined)?;
        let x = self.norm1.forward(&x)?;
        let x = x.relu()?;
        
        let x = self.ln2.forward(&x)?;
        let x = x.relu()?;
        
        let x = self.ln3.forward(&x)?;
        candle_nn::ops::sigmoid(&x)
    }
}

