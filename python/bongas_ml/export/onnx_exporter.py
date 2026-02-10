"""
ONNX Export Module - Converts PyTorch models to ONNX format
"""

import torch
import onnx
import onnxruntime as ort
import numpy as np
from pathlib import Path
import logging
from typing import Dict, Optional, Tuple

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class ONNXExporter:
    """Export PyTorch models to ONNX format with validation"""

    def __init__(self, onnx_dir: str = "models/onnx"):
        self.onnx_dir = Path(onnx_dir)
        self.onnx_dir.mkdir(parents=True, exist_ok=True)

    def export_two_tower(
        self,
        model: torch.nn.Module,
        model_name: str,
        user_feature_dim: int,
        item_feature_dim: int,
        opset_version: int = 17,
    ) -> Dict:
        """
        Export Two-Tower model to ONNX

        Args:
            model: PyTorch model
            model_name: Name for the exported model
            user_feature_dim: Dimension of user features
            item_feature_dim: Dimension of item features
            opset_version: ONNX opset version

        Returns:
            Export metadata dict
        """
        logger.info(f"Exporting {model_name} to ONNX (opset={opset_version})...")

        # Set model to eval mode
        model.eval()
        model.to('cpu')

        # Create dummy inputs for tracing
        batch_size = 1
        dummy_user = torch.randn(batch_size, user_feature_dim)
        dummy_item = torch.randn(batch_size, item_feature_dim)

        # Define output path
        onnx_path = self.onnx_dir / f"{model_name}.onnx"

        # Export to ONNX
        torch.onnx.export(
            model,
            (dummy_user, dummy_item),
            str(onnx_path),
            export_params=True,
            opset_version=opset_version,
            do_constant_folding=True,
            input_names=['user_features', 'item_features'],
            output_names=['scores'],
            dynamic_axes={
                'user_features': {0: 'batch_size'},
                'item_features': {0: 'batch_size'},
                'scores': {0: 'batch_size'},
            },
            verbose=False,
        )

        logger.info(f"Model exported to {onnx_path}")

        # Validate ONNX model
        onnx_model = onnx.load(str(onnx_path))
        onnx.checker.check_model(onnx_model)
        logger.info(f"ONNX model validated")

        # Test inference and compare with PyTorch
        accuracy_match = self.validate_onnx_accuracy(
            pytorch_model=model,
            onnx_path=onnx_path,
            dummy_user=dummy_user,
            dummy_item=dummy_item,
        )

        # Extract metadata
        input_shapes = {}
        output_names = []

        for input_node in onnx_model.graph.input:
            input_shapes[input_node.name] = [
                dim.dim_value if dim.dim_value > 0 else -1
                for dim in input_node.type.tensor_type.shape.dim
            ]

        for output_node in onnx_model.graph.output:
            output_names.append(output_node.name)

        metadata = {
            'model_name': model_name,
            'onnx_path': str(onnx_path),
            'opset_version': opset_version,
            'input_shapes': input_shapes,
            'output_names': output_names,
            'user_feature_dim': user_feature_dim,
            'item_feature_dim': item_feature_dim,
            'accuracy_match': accuracy_match,
        }

        logger.info(f"Export metadata:")
        logger.info(f"   Input shapes: {input_shapes}")
        logger.info(f"   Output names: {output_names}")
        logger.info(f"   Accuracy match: {accuracy_match:.6f}")

        return metadata

    def validate_onnx_accuracy(
        self,
        pytorch_model: torch.nn.Module,
        onnx_path: Path,
        dummy_user: torch.Tensor,
        dummy_item: torch.Tensor,
        tolerance: float = 1e-5,
    ) -> float:
        """
        Validate ONNX model output matches PyTorch

        Returns:
            Max absolute difference between outputs
        """
        logger.info("Validating ONNX accuracy vs PyTorch...")

        # PyTorch inference
        with torch.no_grad():
            pytorch_output = pytorch_model(dummy_user, dummy_item).numpy()

        # ONNX Runtime inference
        ort_session = ort.InferenceSession(str(onnx_path))
        ort_inputs = {
            'user_features': dummy_user.numpy(),
            'item_features': dummy_item.numpy(),
        }
        ort_outputs = ort_session.run(None, ort_inputs)
        onnx_output = ort_outputs[0]

        # Compare outputs
        max_diff = np.abs(pytorch_output - onnx_output).max()

        if max_diff < tolerance:
            logger.info(f"Accuracy validated (max_diff={max_diff:.2e})")
        else:
            logger.warning(f"Accuracy mismatch detected (max_diff={max_diff:.2e})")

        return float(max_diff)


if __name__ == "__main__":
    # Example usage
    from bongas_ml.models.two_tower import TwoTowerModel

    # Create a model
    model = TwoTowerModel(
        user_feature_dim=64,
        item_feature_dim=32,
        embedding_dim=128,
    )

    # Export to ONNX
    exporter = ONNXExporter(onnx_dir="models/onnx")
    metadata = exporter.export_two_tower(
        model=model,
        model_name="two_tower_v1",
        user_feature_dim=64,
        item_feature_dim=32,
    )

    print(f"Export complete: {metadata['onnx_path']}")
