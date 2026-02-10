#!/bin/bash
set -e

echo "=== BONGAS-AI: Train and Export ONNX Models ==="

cd "$(dirname "$0")/../python"

# Step 1: Install dependencies
echo "[1/4] Installing Python dependencies..."
pip install -r requirements.txt

# Step 2: Train models
echo "[2/4] Training models..."
python -m bongas_ml

# Step 3: Export to ONNX
echo "[3/4] Exporting models to ONNX format..."
python -c "
from export.onnx_exporter import export_all_models
export_all_models(output_dir='../models')
"

# Step 4: Validate ONNX output
echo "[4/4] Validating ONNX models..."
python "../scripts/validate_onnx.py" --model-dir "../models"

echo "=== Done. ONNX models saved to models/ ==="
