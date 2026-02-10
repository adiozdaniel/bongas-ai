# ML Integration

## Overview

BONGAS-AI uses a split architecture for ML:
- **Training**: Python/PyTorch (development only, not shipped)
- **Inference**: ONNX Runtime in Rust (production)
- **Bandits**: Native Rust implementation (no Python dependency)

## Supported Models

| Model | Type | Use Case |
|-------|------|----------|
| Two-Tower | Dual encoder | Candidate retrieval |
| BERT4Rec | Transformer | Sequential recommendations |
| NCF | Neural CF | User-item scoring |
| DIN | Attention | Interest-aware ranking |
| Wide & Deep | Hybrid | Combined memorization + generalization |
| AutoInt | Feature interaction | Automatic feature crosses |

## Training Pipeline (Python)

Located in `python/` (dev only):

```
python/
├── models/        # Model definitions (PyTorch)
├── training/      # Training orchestration
├── export/        # ONNX export utilities
├── features/      # Feature engineering
└── utils/         # Metrics, logging
```

### Workflow

```bash
# Train models
python -m bongas_ml

# Export to ONNX
python -c "from export.onnx_exporter import export_all_models; export_all_models('models/')"

# Validate
python scripts/validate_onnx.py --model-dir models/
```

## Inference (Rust)

Located in `src/ml/`:

| File | Purpose |
|------|---------|
| `onnx_runtime.rs` | ONNX Runtime wrapper |
| `model_loader.rs` | Load .onnx files |
| `inference.rs` | Batch/online inference |
| `preprocessing.rs` | Feature preprocessing (Rust-native) |
| `postprocessing.rs` | Score normalization, ranking |
| `feature_store.rs` | Centralized feature management |
| `model_registry.rs` | Versioned model storage |
| `embeddings.rs` | User/item embedding management |
| `online_learning.rs` | Real-time model updates |
| `worker_queue.rs` | Background ML task queue |

## Feature Store

Features are computed hourly by a background worker:

```
ClickHouse (raw events)
    | hourly aggregation
Feature Worker (Rust)
    | compute TF-IDF, embeddings, etc.
PostgreSQL (user_features, item_features)
    | cache hot users
Redis (1hr TTL)
    |
ONNX Runtime
```

## Bandits (Native Rust)

Located in `src/experiments/bandits/`:

- **Thompson Sampling** - Bayesian exploration via Beta distribution (`rand_distr`)
- **UCB** - Upper Confidence Bound (UCB1, UCB-Tuned)
- **LinUCB** - Contextual bandits using linear algebra (`nalgebra`)
- **Epsilon-Greedy** - Simple exploration baseline

No Python dependency needed for experimentation.
