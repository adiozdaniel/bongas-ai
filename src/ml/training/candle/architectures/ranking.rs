//! Native Rust Ranking Head architecture using Candle.

use candle_core::{Result, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

/// A 3-layer MLP for ranking adaptation (The Student Head).
/// It learns to map pre-extracted content DNA to user engagement scores.
pub struct StudentRankingHead {
    ln1: Linear,
    ln2: Linear,
    output: Linear,
}

impl StudentRankingHead {
    /// Create a new ranking head with the specified dimensions.
    pub fn new(vs: VarBuilder) -> Result<Self> {
        let ln1 = candle_nn::linear(512, 256, vs.pp("ln1"))?;
        let ln2 = candle_nn::linear(256, 128, vs.pp("ln2"))?;
        let output = candle_nn::linear(128, 1, vs.pp("output"))?;
        
        Ok(Self { ln1, ln2, output })
    }

    /// Forward pass through the head.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = self.ln1.forward(x)?.relu()?;
        let x = self.ln2.forward(&x)?.relu()?;
        self.output.forward(&x)
    }
}
