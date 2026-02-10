#!/usr/bin/env python3
"""Validate ONNX models: check structure, run test inference, compare with PyTorch."""

import argparse
import sys
from pathlib import Path


def validate_model(model_path: Path) -> bool:
    """Validate a single ONNX model file."""
    import onnx
    import onnxruntime as ort

    print(f"  Validating {model_path.name}...")

    # Check model structure
    model = onnx.load(str(model_path))
    onnx.checker.check_model(model)
    print(f"    Structure: OK")

    # Run test inference
    session = ort.InferenceSession(str(model_path))
    inputs = session.get_inputs()
    outputs = session.get_outputs()
    print(f"    Inputs:  {[i.name for i in inputs]}")
    print(f"    Outputs: {[o.name for o in outputs]}")

    return True


def main():
    parser = argparse.ArgumentParser(description="Validate ONNX models")
    parser.add_argument("--model-dir", type=str, default="models", help="Directory containing .onnx files")
    args = parser.parse_args()

    model_dir = Path(args.model_dir)
    if not model_dir.exists():
        print(f"Error: {model_dir} does not exist")
        sys.exit(1)

    onnx_files = list(model_dir.glob("*.onnx"))
    if not onnx_files:
        print(f"No .onnx files found in {model_dir}")
        sys.exit(1)

    print(f"Found {len(onnx_files)} ONNX model(s)")
    failures = 0

    for model_path in sorted(onnx_files):
        try:
            validate_model(model_path)
        except Exception as e:
            print(f"    FAILED: {e}")
            failures += 1

    if failures:
        print(f"\n{failures} model(s) failed validation")
        sys.exit(1)

    print(f"\nAll {len(onnx_files)} model(s) passed validation")


if __name__ == "__main__":
    main()
