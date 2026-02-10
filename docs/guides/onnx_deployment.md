# ONNX Deployment

## Overview

BONGAS-AI uses ONNX Runtime (`ort` crate) as the primary ML inference engine. Models are trained in Python/PyTorch, exported to `.onnx` format, and served natively in Rust with zero Python dependency at runtime.

## Model Lifecycle

```
[Python Training] --> [ONNX Export] --> [Validation] --> [Deploy .onnx] --> [Rust Inference]
```

### 1. Train (Python)

```python
# python/training/trainer.py
model = TwoTowerModel(config)
trainer.fit(model, train_loader, val_loader)
trainer.save_checkpoint("checkpoints/two_tower.ckpt")
```

### 2. Export to ONNX (Python)

```python
# python/export/onnx_exporter.py
import torch

model = TwoTowerModel.load_from_checkpoint("checkpoints/two_tower.ckpt")
model.eval()

dummy_input = torch.randn(1, 256)
torch.onnx.export(
    model,
    dummy_input,
    "models/two_tower_v1.onnx",
    opset_version=17,
    input_names=["user_features"],
    output_names=["scores"],
    dynamic_axes={"user_features": {0: "batch_size"}, "scores": {0: "batch_size"}},
)
```

### 3. Validate (Python)

```bash
python scripts/validate_onnx.py --model-dir models/
```

Checks:
- ONNX model structure validity
- Input/output shapes
- Test inference runs without error
- Output matches PyTorch within tolerance

### 4. Deploy

Place `.onnx` files in the `models/` directory:

```
models/
├── two_tower_v1.onnx
├── bert4rec_v1.onnx
├── ncf_v1.onnx
└── din_v1.onnx
```

### 5. Rust Inference

The `ort` crate loads and runs models natively:

```rust
// src/ml/onnx_runtime.rs
let session = ort::Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .commit_from_file("models/two_tower_v1.onnx")?;

let outputs = session.run(ort::inputs![user_features]?)?;
```

## Configuration

```toml
# config/default.toml
[onnx]
enabled = true
model_path = "./models"
execution_provider = "cpu"    # or "cuda"
graph_optimization = true
```

## Model Versioning

Models follow the naming convention `{model_name}_v{version}.onnx`. The model registry tracks which version is active for each pipeline stage.

## Packaging

Production package includes only:
- Rust binary (`bongas-ai`)
- ONNX model files (`models/*.onnx`)
- Configuration (`config/default.toml`)

No Python runtime, no `.py` files, no training code.

```bash
./scripts/package.sh 1.0.0
# Output: dist/bongas-ai-v1.0.0-linux-x64.tar.gz
```
