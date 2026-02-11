"""
Unit tests for ONNX export functionality in BONGAS-AI
"""

import pytest
import torch
import onnx
import onnxruntime as ort
import numpy as np
import tempfile
import os
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock

from bongas_ml.export.onnx_exporter import ONNXExporter
from bongas_ml.models.two_tower import TwoTowerModel
from tests.python.fixtures import (
    TestConfig, create_synthetic_user_features, create_synthetic_item_features,
    create_temp_onnx_path, get_test_device
)


class TestONNXExporter:
    """Test cases for ONNXExporter"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.exporter = ONNXExporter(onnx_dir=self.temp_dir)
        
        # Create a test model
        self.model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
        )
        self.model.eval()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_exporter_initialization(self):
        """Test ONNXExporter initialization"""
        exporter = ONNXExporter(onnx_dir="test_onnx")
        assert exporter.onnx_dir.exists()
        assert exporter.onnx_dir.name == "test_onnx"

    def test_export_two_tower_basic(self):
        """Test basic ONNX export functionality"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            opset_version=17,
        )
        
        # Check metadata
        assert metadata['model_name'] == "test_model"
        assert 'onnx_path' in metadata
        assert metadata['opset_version'] == 17
        assert metadata['user_feature_dim'] == TestConfig.USER_FEATURE_DIM
        assert metadata['item_feature_dim'] == TestConfig.ITEM_FEATURE_DIM
        
        # Check that ONNX file was created
        onnx_path = Path(metadata['onnx_path'])
        assert onnx_path.exists()
        
        # Check ONNX model validity
        onnx_model = onnx.load(str(onnx_path))
        onnx.checker.check_model(onnx_model)

    def test_export_two_tower_different_opset_versions(self):
        """Test export with different ONNX opset versions"""
        for opset_version in [14, 15, 16, 17]:
            metadata = self.exporter.export_two_tower(
                model=self.model,
                model_name=f"test_model_v{opset_version}",
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                opset_version=opset_version,
            )
            
            assert metadata['opset_version'] == opset_version
            
            # Verify ONNX file
            onnx_path = Path(metadata['onnx_path'])
            assert onnx_path.exists()
            
            onnx_model = onnx.load(str(onnx_path))
            onnx.checker.check_model(onnx_model)

    def test_export_two_tower_different_dimensions(self):
        """Test export with different feature dimensions"""
        test_cases = [
            (64, 32),
            (100, 50),
            (128, 64),
        ]
        
        for user_dim, item_dim in test_cases:
            model = TwoTowerModel(
                user_feature_dim=user_dim,
                item_feature_dim=item_dim,
                embedding_dim=128,
            )
            model.eval()
            
            metadata = self.exporter.export_two_tower(
                model=model,
                model_name=f"test_model_{user_dim}_{item_dim}",
                user_feature_dim=user_dim,
                item_feature_dim=item_dim,
            )
            
            assert metadata['user_feature_dim'] == user_dim
            assert metadata['item_feature_dim'] == item_dim
            
            # Verify input shapes
            input_shapes = metadata['input_shapes']
            assert input_shapes['user_features'][1] == user_dim
            assert input_shapes['item_features'][1] == item_dim

    def test_export_two_tower_model_state(self):
        """Test that model state is preserved after export"""
        # Get original model state
        original_state = self.model.state_dict()
        
        # Export model
        self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Check that model state is unchanged
        new_state = self.model.state_dict()
        
        for key in original_state:
            assert torch.equal(original_state[key], new_state[key])

    def test_export_two_tower_model_device_consistency(self):
        """Test export works on different devices"""
        if torch.cuda.is_available():
            model_cuda = self.model.cuda()
            
            metadata = self.exporter.export_two_tower(
                model=model_cuda,
                model_name="test_model_cuda",
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            )
            
            # Verify ONNX file was created
            onnx_path = Path(metadata['onnx_path'])
            assert onnx_path.exists()

    def test_export_two_tower_input_output_names(self):
        """Test that input and output names are correct"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Check input and output names
        assert 'user_features' in metadata['input_shapes']
        assert 'item_features' in metadata['input_shapes']
        assert 'scores' in metadata['output_names']
        
        # Check dynamic axes
        onnx_path = Path(metadata['onnx_path'])
        onnx_model = onnx.load(str(onnx_path))
        
        # Verify dynamic batch size
        for input_node in onnx_model.graph.input:
            if input_node.name in ['user_features', 'item_features']:
                # First dimension should be dynamic (batch_size)
                assert input_node.type.tensor_type.shape.dim[0].dim_param == 'batch_size'

    def test_export_two_tower_accuracy_validation(self):
        """Test that ONNX accuracy validation works"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Check that accuracy validation was performed
        assert 'accuracy_match' in metadata
        assert isinstance(metadata['accuracy_match'], float)
        
        # Accuracy should be very good (close to zero difference)
        assert metadata['accuracy_match'] < 1e-4

    def test_export_two_tower_onnx_validation(self):
        """Test ONNX model validation"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Load and validate ONNX model
        onnx_path = Path(metadata['onnx_path'])
        onnx_model = onnx.load(str(onnx_path))
        
        # Should not raise any validation errors
        onnx.checker.check_model(onnx_model)

    def test_export_two_tower_onnx_inference(self):
        """Test ONNX model inference"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        onnx_path = Path(metadata['onnx_path'])
        
        # Test ONNX Runtime inference
        ort_session = ort.InferenceSession(str(onnx_path))
        
        # Create test inputs
        batch_size = 3
        user_features = np.random.randn(batch_size, TestConfig.USER_FEATURE_DIM).astype(np.float32)
        item_features = np.random.randn(batch_size, TestConfig.ITEM_FEATURE_DIM).astype(np.float32)
        
        # Run inference
        ort_inputs = {
            'user_features': user_features,
            'item_features': item_features,
        }
        ort_outputs = ort_session.run(None, ort_inputs)
        
        # Check output
        assert len(ort_outputs) == 1
        scores = ort_outputs[0]
        assert scores.shape == (batch_size,)
        assert scores.dtype == np.float32

    def test_export_two_tower_pytorch_vs_onnx_comparison(self):
        """Test that PyTorch and ONNX outputs are similar"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        onnx_path = Path(metadata['onnx_path'])
        
        # Create test inputs
        batch_size = 5
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # PyTorch inference
        with torch.no_grad():
            pytorch_scores = self.model(user_features, item_features).numpy()
        
        # ONNX inference
        ort_session = ort.InferenceSession(str(onnx_path))
        ort_inputs = {
            'user_features': user_features.numpy(),
            'item_features': item_features.numpy(),
        }
        ort_outputs = ort_session.run(None, ort_inputs)
        onnx_scores = ort_outputs[0]
        
        # Compare outputs
        max_diff = np.abs(pytorch_scores - onnx_scores).max()
        assert max_diff < 1e-4, f"PyTorch vs ONNX difference too large: {max_diff}"

    def test_export_two_tower_different_batch_sizes(self):
        """Test ONNX model with different batch sizes"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        onnx_path = Path(metadata['onnx_path'])
        ort_session = ort.InferenceSession(str(onnx_path))
        
        # Test different batch sizes
        for batch_size in [1, 5, 10, 20]:
            user_features = np.random.randn(batch_size, TestConfig.USER_FEATURE_DIM).astype(np.float32)
            item_features = np.random.randn(batch_size, TestConfig.ITEM_FEATURE_DIM).astype(np.float32)
            
            ort_inputs = {
                'user_features': user_features,
                'item_features': item_features,
            }
            ort_outputs = ort_session.run(None, ort_inputs)
            
            scores = ort_outputs[0]
            assert scores.shape == (batch_size,)

    def test_export_two_tower_model_name_validation(self):
        """Test model name validation"""
        # Test with valid names
        valid_names = ["model_v1", "test_model", "my_model_123"]
        
        for name in valid_names:
            metadata = self.exporter.export_two_tower(
                model=self.model,
                model_name=name,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            )
            
            assert metadata['model_name'] == name
            onnx_path = Path(metadata['onnx_path'])
            assert onnx_path.name == f"{name}.onnx"

    def test_export_two_tower_file_permissions(self):
        """Test that exported files have correct permissions"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        onnx_path = Path(metadata['onnx_path'])
        
        # File should be readable
        assert onnx_path.is_file()
        assert os.access(onnx_path, os.R_OK)


class TestONNXExporterEdgeCases:
    """Test edge cases for ONNXExporter"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.exporter = ONNXExporter(onnx_dir=self.temp_dir)
        
        # Create a simple test model
        self.model = TwoTowerModel(
            user_feature_dim=10,
            item_feature_dim=5,
            embedding_dim=16,
            hidden_dims=[32, 16],
        )
        self.model.eval()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_export_with_minimal_dimensions(self):
        """Test export with minimal feature dimensions"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="minimal_model",
            user_feature_dim=10,
            item_feature_dim=5,
        )
        
        assert metadata['user_feature_dim'] == 10
        assert metadata['item_feature_dim'] == 5
        
        # Verify ONNX file
        onnx_path = Path(metadata['onnx_path'])
        assert onnx_path.exists()

    def test_export_with_large_dimensions(self):
        """Test export with large feature dimensions"""
        large_model = TwoTowerModel(
            user_feature_dim=512,
            item_feature_dim=256,
            embedding_dim=256,
            hidden_dims=[512, 256],
        )
        large_model.eval()
        
        metadata = self.exporter.export_two_tower(
            model=large_model,
            model_name="large_model",
            user_feature_dim=512,
            item_feature_dim=256,
        )
        
        assert metadata['user_feature_dim'] == 512
        assert metadata['item_feature_dim'] == 256

    def test_export_with_custom_opset_version(self):
        """Test export with custom opset version"""
        metadata = self.exporter.export_two_tower(
            model=self.model,
            model_name="custom_opset",
            user_feature_dim=10,
            item_feature_dim=5,
            opset_version=16,
        )
        
        assert metadata['opset_version'] == 16
        
        # Verify ONNX file
        onnx_path = Path(metadata['onnx_path'])
        onnx_model = onnx.load(str(onnx_path))
        assert onnx_model.opset_import[0].version == 16

    def test_export_multiple_models(self):
        """Test exporting multiple models to the same directory"""
        models = []
        for i in range(3):
            model = TwoTowerModel(
                user_feature_dim=10 + i,
                item_feature_dim=5 + i,
                embedding_dim=16 + i,
            )
            model.eval()
            models.append(model)
        
        for i, model in enumerate(models):
            metadata = self.exporter.export_two_tower(
                model=model,
                model_name=f"model_{i}",
                user_feature_dim=10 + i,
                item_feature_dim=5 + i,
            )
            
            assert metadata['model_name'] == f"model_{i}"
            
            # Verify all files exist
            onnx_path = Path(metadata['onnx_path'])
            assert onnx_path.exists()

    def test_export_with_invalid_opset_version(self):
        """Test export with invalid opset version"""
        with pytest.raises(Exception):  # Should raise ONNX export error
            self.exporter.export_two_tower(
                model=self.model,
                model_name="invalid_opset",
                user_feature_dim=10,
                item_feature_dim=5,
                opset_version=999,  # Invalid opset version
            )


class TestONNXExporterIntegration:
    """Integration tests for ONNXExporter"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.exporter = ONNXExporter(onnx_dir=self.temp_dir)

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_complete_export_workflow(self):
        """Test complete export workflow"""
        # Create and train a simple model
        model = TwoTowerModel(
            user_feature_dim=32,
            item_feature_dim=16,
            embedding_dim=64,
        )
        model.eval()
        
        # Export model
        metadata = self.exporter.export_two_tower(
            model=model,
            model_name="workflow_test",
            user_feature_dim=32,
            item_feature_dim=16,
        )
        
        # Verify export
        assert metadata['model_name'] == "workflow_test"
        assert 'onnx_path' in metadata
        
        # Verify ONNX file
        onnx_path = Path(metadata['onnx_path'])
        assert onnx_path.exists()
        
        # Verify ONNX model
        onnx_model = onnx.load(str(onnx_path))
        onnx.checker.check_model(onnx_model)
        
        # Verify inference
        ort_session = ort.InferenceSession(str(onnx_path))
        
        user_features = np.random.randn(2, 32).astype(np.float32)
        item_features = np.random.randn(2, 16).astype(np.float32)
        
        ort_inputs = {
            'user_features': user_features,
            'item_features': item_features,
        }
        ort_outputs = ort_session.run(None, ort_inputs)
        
        assert len(ort_outputs) == 1
        assert ort_outputs[0].shape == (2,)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])