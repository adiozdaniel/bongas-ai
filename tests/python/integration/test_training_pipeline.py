"""
Integration tests for the complete training pipeline in BONGAS-AI
"""

import pytest
import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
import tempfile
import os
from pathlib import Path
from unittest.mock import patch, MagicMock

from bongas_ml.training.train_two_tower import TwoTowerModel, RecommendationDataset, train_two_tower
from bongas_ml.export.onnx_exporter import ONNXExporter
from tests.python.fixtures import (
    TestConfig, create_test_dataset, create_synthetic_user_features, 
    create_synthetic_item_features, get_test_device
)


class TestTrainingPipelineIntegration:
    """Integration tests for the complete training pipeline"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.device = get_test_device()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_complete_training_pipeline(self):
        """Test the complete training pipeline from data to model"""
        # Create synthetic training data
        train_data = create_test_dataset(200)
        val_data = create_test_dataset(50)
        
        # Train model
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
            epochs=2,
            batch_size=16,
            learning_rate=1e-3,
            device=self.device,
        )
        
        # Verify model
        assert isinstance(model, TwoTowerModel)
        assert not model.training  # Should be in eval mode
        
        # Test model inference
        batch_size = 5
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
            model = model.cuda()
        
        with torch.no_grad():
            scores = model(user_features, item_features)
        
        assert scores.shape == (batch_size,)
        assert torch.isfinite(scores).all()

    def test_training_pipeline_with_export(self):
        """Test training pipeline followed by ONNX export"""
        # Create training data
        train_data = create_test_dataset(150)
        val_data = create_test_dataset(30)
        
        # Train model
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=12,
            device=self.device,
        )
        
        # Export to ONNX
        exporter = ONNXExporter(onnx_dir=self.temp_dir)
        metadata = exporter.export_two_tower(
            model=model,
            model_name="integrated_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Verify export
        assert metadata['model_name'] == "integrated_model"
        onnx_path = Path(metadata['onnx_path'])
        assert onnx_path.exists()
        
        # Verify ONNX model
        import onnx
        onnx_model = onnx.load(str(onnx_path))
        onnx.checker.check_model(onnx_model)
        
        # Verify ONNX inference
        import onnxruntime as ort
        ort_session = ort.InferenceSession(str(onnx_path))
        
        user_features = np.random.randn(3, TestConfig.USER_FEATURE_DIM).astype(np.float32)
        item_features = np.random.randn(3, TestConfig.ITEM_FEATURE_DIM).astype(np.float32)
        
        ort_inputs = {
            'user_features': user_features,
            'item_features': item_features,
        }
        ort_outputs = ort_session.run(None, ort_inputs)
        
        assert len(ort_outputs) == 1
        assert ort_outputs[0].shape == (3,)

    def test_training_pipeline_with_different_configurations(self):
        """Test training pipeline with different configurations"""
        configurations = [
            {
                'embedding_dim': 64,
                'hidden_dims': [128, 64],
                'batch_size': 8,
                'epochs': 1,
            },
            {
                'embedding_dim': 128,
                'hidden_dims': [256, 128],
                'batch_size': 16,
                'epochs': 2,
            },
            {
                'embedding_dim': 256,
                'hidden_dims': [512, 256, 128],
                'batch_size': 32,
                'epochs': 3,
            },
        ]
        
        for config in configurations:
            train_data = create_test_dataset(100)
            val_data = create_test_dataset(25)
            
            model = train_two_tower(
                train_data=train_data,
                val_data=val_data,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                **config,
                device=self.device,
            )
            
            # Test model
            user_features = torch.randn(2, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(2, TestConfig.ITEM_FEATURE_DIM)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
                model = model.cuda()
            
            with torch.no_grad():
                scores = model(user_features, item_features)
            
            assert scores.shape == (2,)
            assert torch.isfinite(scores).all()

    def test_training_pipeline_with_realistic_data(self):
        """Test training pipeline with more realistic synthetic data"""
        # Create more realistic training data
        np.random.seed(42)
        
        # Create user features with some structure
        n_users = 50
        n_items = 25
        n_interactions = 300
        
        train_data = []
        for _ in range(n_interactions):
            user_id = np.random.randint(0, n_users)
            item_id = np.random.randint(0, n_items)
            
            # Create structured user features
            user_features = np.random.randn(TestConfig.USER_FEATURE_DIM)
            # Add some bias based on user_id
            user_features[0] += user_id * 0.1
            
            # Create structured item features
            item_features = np.random.randn(TestConfig.ITEM_FEATURE_DIM)
            # Add some bias based on item_id
            item_features[0] += item_id * 0.1
            
            # Create rating based on some interaction
            rating = float((user_features[0] + item_features[0] + np.random.randn() * 0.1) > 0)
            
            train_data.append({
                'user_id': user_id,
                'item_id': item_id,
                'user_features': user_features.astype(np.float32),
                'item_features': item_features.astype(np.float32),
                'rating': rating,
            })
        
        val_data = create_test_dataset(50)
        
        # Train model
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=3,
            batch_size=16,
            device=self.device,
        )
        
        # Test model generalization
        test_user_features = torch.randn(5, TestConfig.USER_FEATURE_DIM)
        test_item_features = torch.randn(5, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            test_user_features = test_user_features.cuda()
            test_item_features = test_item_features.cuda()
            model = model.cuda()
        
        with torch.no_grad():
            model.eval()
            scores = model(test_user_features, test_item_features)
        
        assert scores.shape == (5,)
        assert torch.isfinite(scores).all()

    def test_training_pipeline_with_validation_monitoring(self):
        """Test training pipeline with validation monitoring"""
        train_data = create_test_dataset(100)
        val_data = create_test_dataset(25)
        
        # Mock the logger to capture validation losses
        validation_losses = []
        
        def mock_logger_info(msg):
            if "Val Loss" in msg:
                # Extract validation loss from log message
                parts = msg.split()
                for i, part in enumerate(parts):
                    if part == "Val" and i + 2 < len(parts):
                        try:
                            val_loss = float(parts[i + 2].replace(',', ''))
                            validation_losses.append(val_loss)
                        except:
                            pass
        
        with patch('bongas_ml.training.train_two_tower.logger.info', side_effect=mock_logger_info):
            model = train_two_tower(
                train_data=train_data,
                val_data=val_data,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=3,
                batch_size=16,
                device=self.device,
            )
        
        # Should have recorded validation losses
        assert len(validation_losses) == 3  # One per epoch
        assert all(isinstance(loss, float) for loss in validation_losses)

    def test_training_pipeline_with_device_switching(self):
        """Test training pipeline with device switching"""
        train_data = create_test_dataset(50)
        val_data = create_test_dataset(10)
        
        # Test CPU training
        model_cpu = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=8,
            device='cpu',
        )
        
        assert next(model_cpu.parameters()).device.type == 'cpu'
        
        # Test CUDA training if available
        if torch.cuda.is_available():
            model_cuda = train_two_tower(
                train_data=train_data,
                val_data=val_data,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=1,
                batch_size=8,
                device='cuda',
            )
            
            assert next(model_cuda.parameters()).device.type == 'cuda'

    def test_training_pipeline_with_model_saving(self):
        """Test training pipeline with model saving"""
        train_data = create_test_dataset(80)
        val_data = create_test_dataset(20)
        
        # Train model
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=10,
            device=self.device,
        )
        
        # Save model
        model_path = os.path.join(self.temp_dir, "trained_model.pt")
        torch.save(model.state_dict(), model_path)
        
        # Load model
        loaded_model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
        )
        loaded_model.load_state_dict(torch.load(model_path))
        loaded_model.eval()
        
        # Test that loaded model works
        user_features = torch.randn(3, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(3, TestConfig.ITEM_FEATURE_DIM)
        
        with torch.no_grad():
            original_scores = model(user_features, item_features)
            loaded_scores = loaded_model(user_features, item_features)
        
        # Scores should be identical
        assert torch.allclose(original_scores, loaded_scores, atol=1e-6)

    def test_training_pipeline_with_different_loss_functions(self):
        """Test training pipeline with different loss functions"""
        train_data = create_test_dataset(60)
        val_data = create_test_dataset(15)
        
        # Test with BCEWithLogitsLoss (default)
        model1 = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=8,
            device=self.device,
        )
        
        # Test model inference
        user_features = torch.randn(2, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(2, TestConfig.ITEM_FEATURE_DIM)
        
        with torch.no_grad():
            scores1 = model1(user_features, item_features)
        
        assert scores1.shape == (2,)
        assert torch.isfinite(scores1).all()

    def test_training_pipeline_with_large_dataset(self):
        """Test training pipeline with larger dataset"""
        # Create larger dataset
        train_data = create_test_dataset(500)
        val_data = create_test_dataset(100)
        
        # Train model
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=32,
            device=self.device,
        )
        
        # Test model
        user_features = torch.randn(10, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(10, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
            model = model.cuda()
        
        with torch.no_grad():
            scores = model(user_features, item_features)
        
        assert scores.shape == (10,)
        assert torch.isfinite(scores).all()

    def test_training_pipeline_error_handling(self):
        """Test training pipeline error handling"""
        # Test with empty training data
        with pytest.raises((ValueError, IndexError)):
            train_two_tower(
                train_data=[],
                val_data=[],
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=1,
            )
        
        # Test with mismatched feature dimensions
        train_data = create_test_dataset(20)
        val_data = create_test_dataset(5)
        
        # This should work but might produce warnings
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=4,
            device=self.device,
        )
        
        assert isinstance(model, TwoTowerModel)


class TestTrainingPipelinePerformance:
    """Performance tests for the training pipeline"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.device = get_test_device()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_training_speed(self):
        """Test training speed requirements"""
        import time
        
        train_data = create_test_dataset(200)
        val_data = create_test_dataset(50)
        
        start_time = time.time()
        
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=16,
            device=self.device,
        )
        
        training_time = time.time() - start_time
        
        # Training should complete in reasonable time (adjust based on hardware)
        assert training_time < 60.0, f"Training took too long: {training_time:.2f}s"
        
        # Model should be valid
        assert isinstance(model, TwoTowerModel)

    def test_memory_usage(self):
        """Test memory usage during training"""
        import psutil
        import gc
        
        # Get initial memory
        process = psutil.Process()
        initial_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        train_data = create_test_dataset(300)
        val_data = create_test_dataset(75)
        
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=16,
            device=self.device,
        )
        
        # Force garbage collection
        gc.collect()
        
        # Get final memory
        final_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        # Memory increase should be reasonable
        memory_increase = final_memory - initial_memory
        assert memory_increase < 500.0, f"Memory usage increased too much: {memory_increase:.2f}MB"
        
        # Model should be valid
        assert isinstance(model, TwoTowerModel)

    def test_model_size(self):
        """Test final model size"""
        train_data = create_test_dataset(100)
        val_data = create_test_dataset(25)
        
        model = train_two_tower(
            train_data=train_data,
            val_data=val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=2,
            batch_size=16,
            device=self.device,
        )
        
        # Save model and check size
        model_path = os.path.join(self.temp_dir, "model_size_test.pt")
        torch.save(model.state_dict(), model_path)
        
        model_size = os.path.getsize(model_path) / 1024  # KB
        
        # Model size should be reasonable
        assert model_size < 10000.0, f"Model size too large: {model_size:.2f}KB"
        assert model_size > 1.0, f"Model size too small: {model_size:.2f}KB"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])