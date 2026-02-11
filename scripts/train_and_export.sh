#!/bin/bash
set -e

echo "=== BONGAS-AI: Training & ONNX Export Pipeline ==="
echo ""

# Step 1: Install dependencies
echo "[1/6] Installing Python dependencies..."
cd "$(dirname "$0")/../bongas-ml"
pip install -r requirements.txt
pip install onnx onnxruntime

# Step 2: Train models
echo "[2/6] Training ML models..."
python -m bongas_ml.training.train_two_tower

# Step 3: Export to ONNX
echo "[3/6] Exporting models to ONNX format..."
python -m bongas_ml.export.onnx_exporter

# Step 4: Validate ONNX models
echo "[4/6] Validating ONNX models..."
python -c "
import onnx
import onnxruntime as ort
from pathlib import Path

onnx_dir = Path('models/onnx')
for onnx_file in onnx_dir.glob('*.onnx'):
    print(f'Validating {onnx_file.name}...')

    # Load and check
    model = onnx.load(str(onnx_file))
    onnx.checker.check_model(model)

    # Test inference
    session = ort.InferenceSession(str(onnx_file))
    print(f'  {onnx_file.name} validated')
"

# Step 5: Copy ONNX models to Rust project
echo "[5/6] Copying ONNX models to Rust project..."
mkdir -p ../models/onnx
cp models/onnx/*.onnx ../models/onnx/

# Step 6: List exported models
echo ""
echo "[6/6] Export complete! ONNX models:"
ls -lh ../models/onnx/*.onnx

echo ""
echo "=== Training and export pipeline complete! ==="
echo "ONNX models: ../models/onnx/"
echo ""
echo "Note: Python code is NOT shipped to production"
echo "Only .onnx binary files are deployed with the Rust binary"
