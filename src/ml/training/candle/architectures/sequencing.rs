//! Native Rust Sequencing Head architecture using Candle.

use candle_core::{Result, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

/// A 3-layer MLP for session-aware sequencing (M19 adaptation).
pub struct StudentSequenceHead {
    ln1: Linear,
    ln2: Linear,
    output: Linear,
}

impl StudentSequenceHead {
    pub fn new(vs: VarBuilder) -> Result<Self> {
        let ln1 = candle_nn::linear(768, 256, vs.pp("ln1"))?;
        let ln2 = candle_nn::linear(256, 128, vs.pp("ln2"))?;
        let output = candle_nn::linear(128, 768, vs.pp("output"))?;
        
        Ok(Self { ln1, ln2, output })
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = self.ln1.forward(x)?.relu()?;
        let x = self.ln2.forward(&x)?.relu()?;
        self.output.forward(&x)
    }
}
