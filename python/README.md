# bongas-ml

Training-only Python package for BONGAS-AI ML models. This code is **not shipped** to production — trained models are exported to ONNX format and served via the Rust ONNX Runtime.

## Usage

```bash
pip install -r requirements.txt
python -m bongas_ml
```

## Workflow

1. Train models using PyTorch (`bongas_ml/training/`)
2. Export to ONNX (`bongas_ml/export/onnx_exporter.py`)
3. Validate ONNX output matches PyTorch (`bongas_ml/export/validate.py`)
4. Optimize ONNX graph (`bongas_ml/export/optimize.py`)
5. Place `.onnx` files in `models/` directory for Rust runtime
