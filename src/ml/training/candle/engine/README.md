# ⚙️ Candle Training Engine

The core logic for native Rust training loops.

## 🚀 Services

### `RankingTrainer`
Orchestrates the training of the `StudentRankingHead`. It handles:
- **Forward Pass**: Native inference.
- **Loss Calculation**: Sovereign MSE/BCE.
- **Backward Pass**: Automatic differentiation using Candle.
- **Optimization**: AdamW weight updates.
