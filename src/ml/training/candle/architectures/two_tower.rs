//! Native Rust Two-Tower architecture using Candle.

use candle_core::{Result, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

/// A single tower in the Two-Tower architecture.
pub struct Tower {
    layers: Vec<Linear>,
}

impl Tower {
    pub fn new(input_dim: usize, hidden_dims: &[usize], output_dim: usize, vs: VarBuilder) -> Result<Self> {
        let mut layers = Vec::new();
        let mut prev_dim = input_dim;
        
        for (i, &hidden_dim) in hidden_dims.iter().enumerate() {
            // Layer names match the Sequential structure in PyTorch: mlp.0, mlp.4, etc.
            // Simplified for now to match basic linear stacking.
            layers.push(candle_nn::linear(prev_dim, hidden_dim, vs.pp(format!("layer_{}", i)))?);
            prev_dim = hidden_dim;
        }
        
        layers.push(candle_nn::linear(prev_dim, output_dim, vs.pp("output"))?);
        
        Ok(Self { layers })
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut x = x.clone();
        for (i, layer) in self.layers.iter().enumerate() {
            x = layer.forward(&x)?;
            if i < self.layers.len() - 1 {
                x = x.relu()?;
            }
        }
        Ok(x)
    }
}

/// The Grand Two-Tower Architecture.
pub struct TwoTowerModel {
    user_tower: Tower,
    item_tower: Tower,
    normalize: bool,
}

impl TwoTowerModel {
    pub fn new(vs: VarBuilder) -> Result<Self> {
        let user_tower = Tower::new(128, &[256, 128], 128, vs.pp("user_tower"))?;
        let item_tower = Tower::new(128, &[256, 128], 128, vs.pp("item_tower"))?;
        
        Ok(Self {
            user_tower,
            item_tower,
            normalize: true,
        })
    }

    pub fn forward(&self, user_features: &Tensor, item_features: &Tensor) -> Result<Tensor> {
        let user_emb = self.user_tower.forward(user_features)?;
        let item_emb = self.item_tower.forward(item_features)?;
        
        let (user_emb, item_emb) = if self.normalize {
            (normalize_l2(&user_emb)?, normalize_l2(&item_emb)?)
        } else {
            (user_emb, item_emb)
        };

        // Cosine similarity: dot product of normalized embeddings
        let dot = (user_emb * item_emb)?.sum_keepdim(1)?;
        Ok(dot)
    }
}

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    let norm = v.sqr()?.sum_keepdim(1)?.sqrt()?;
    v.broadcast_div(&norm)
}
