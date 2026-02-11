"""
Unit tests for ML models in BONGAS-AI
"""

import pytest
import torch
import torch.nn as nn
import numpy as np
from unittest.mock import Mock, patch
import tempfile
import os

from bongas_ml.models.two_tower import TwoTowerModel
from tests.python.fixtures import (
    TestConfig, create_synthetic_user_features, create_synthetic_item_features,
    create_test_dataset, assert_tensors_close, assert_shapes_match, get_test_device
)


class TestTwoTowerModel:
    """Test cases for TwoTowerModel"""

    def setup_method(self):
        """Setup for each test method"""
        self.model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
        )
        self.device = get_test_device()

    def test_model_initialization(self):
        """Test model initialization with default parameters"""
        model = TwoTowerModel(
            user_feature_dim=64,
            item_feature_dim=32,
            embedding_dim=128,
        )
        
        # Check model structure
        assert isinstance(model.user_tower, nn.Sequential)
        assert isinstance(model.item_tower, nn.Sequential)
        
        # Check embedding dimensions
        user_output_layer = model.user_tower[-1]
        item_output_layer = model.item_tower[-1]
        
        assert user_output_layer.out_features == 128
        assert item_output_layer.out_features == 128

    def test_model_initialization_custom_params(self):
        """Test model initialization with custom parameters"""
        model = TwoTowerModel(
            user_feature_dim=100,
            item_feature_dim=50,
            embedding_dim=64,
            hidden_dims=[200, 100],
            dropout=0.5,
        )
        
        # Check input dimensions
        user_input_layer = model.user_tower[0]
        item_input_layer = model.item_tower[0]
        
        assert user_input_layer.in_features == 100
        assert item_input_layer.in_features == 50

    def test_forward_pass(self):
        """Test forward pass with valid inputs"""
        batch_size = 4
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Forward pass
        scores = self.model(user_features, item_features)
        
        # Check output shape and type
        assert scores.shape == (batch_size,)
        assert scores.dtype == torch.float32
        assert scores.device == user_features.device

    def test_forward_pass_device_consistency(self):
        """Test that model works on different devices"""
        if torch.cuda.is_available():
            model = self.model.to('cuda')
            user_features = torch.randn(2, TestConfig.USER_FEATURE_DIM).cuda()
            item_features = torch.randn(2, TestConfig.ITEM_FEATURE_DIM).cuda()
            
            scores = model(user_features, item_features)
            assert scores.device.type == 'cuda'

    def test_encode_user(self):
        """Test user encoding method"""
        batch_size = 3
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        
        user_emb = self.model.encode_user(user_features)
        
        # Check output shape and normalization
        assert_shapes_match(user_emb, (batch_size, TestConfig.EMBEDDING_DIM))
        
        # Check normalization (L2 norm should be 1)
        norms = torch.norm(user_emb, p=2, dim=1)
        assert_tensors_close(norms, torch.ones(batch_size))

    def test_encode_item(self):
        """Test item encoding method"""
        batch_size = 3
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        item_emb = self.model.encode_item(item_features)
        
        # Check output shape and normalization
        assert_shapes_match(item_emb, (batch_size, TestConfig.EMBEDDING_DIM))
        
        # Check normalization (L2 norm should be 1)
        norms = torch.norm(item_emb, p=2, dim=1)
        assert_tensors_close(norms, torch.ones(batch_size))

    def test_embedding_normalization(self):
        """Test that embeddings are properly normalized"""
        batch_size = 5
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Get raw embeddings before normalization
        with torch.no_grad():
            user_emb_raw = self.model.user_tower(user_features)
            item_emb_raw = self.model.item_tower(item_features)
        
        # Get normalized embeddings
        user_emb_norm = self.model.encode_user(user_features)
        item_emb_norm = self.model.encode_item(item_features)
        
        # Check that normalized embeddings have unit norm
        user_norms = torch.norm(user_emb_norm, p=2, dim=1)
        item_norms = torch.norm(item_emb_norm, p=2, dim=1)
        
        assert_tensors_close(user_norms, torch.ones(batch_size))
        assert_tensors_close(item_norms, torch.ones(batch_size))

    def test_similarity_computation(self):
        """Test similarity computation between user and item embeddings"""
        batch_size = 4
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Forward pass
        scores = self.model(user_features, item_features)
        
        # Manual computation
        user_emb = self.model.encode_user(user_features)
        item_emb = self.model.encode_item(item_features)
        manual_scores = (user_emb * item_emb).sum(dim=1)
        
        # Should be identical
        assert_tensors_close(scores, manual_scores)

    def test_batch_processing(self):
        """Test model with different batch sizes"""
        for batch_size in [1, 8, 16, 32]:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            scores = self.model(user_features, item_features)
            
            assert scores.shape == (batch_size,)
            assert torch.isfinite(scores).all()

    def test_gradient_flow(self):
        """Test that gradients flow properly through the model"""
        batch_size = 4
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM, requires_grad=True)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM, requires_grad=True)
        
        scores = self.model(user_features, item_features)
        loss = scores.sum()
        loss.backward()
        
        # Check that gradients exist
        assert user_features.grad is not None
        assert item_features.grad is not None
        assert torch.isfinite(user_features.grad).all()
        assert torch.isfinite(item_features.grad).all()

    def test_model_parameters(self):
        """Test that model has learnable parameters"""
        params = list(self.model.parameters())
        assert len(params) > 0
        
        # Check that parameters require gradients
        for param in params:
            assert param.requires_grad
            assert param.grad is None  # Initially no gradients

    def test_model_state_dict(self):
        """Test model state dict operations"""
        # Get state dict
        state_dict = self.model.state_dict()
        assert len(state_dict) > 0
        
        # Check key structure
        user_keys = [k for k in state_dict.keys() if 'user_tower' in k]
        item_keys = [k for k in state_dict.keys() if 'item_tower' in k]
        
        assert len(user_keys) > 0
        assert len(item_keys) > 0

    def test_model_load_state_dict(self):
        """Test loading model state dict"""
        # Create a new model
        new_model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
        )
        
        # Load state dict
        state_dict = self.model.state_dict()
        new_model.load_state_dict(state_dict)
        
        # Test that outputs are identical
        batch_size = 2
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        with torch.no_grad():
            original_scores = self.model(user_features, item_features)
            loaded_scores = new_model(user_features, item_features)
        
        assert_tensors_close(original_scores, loaded_scores)

    def test_model_eval_mode(self):
        """Test model behavior in eval mode"""
        self.model.eval()
        
        batch_size = 4
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Should work in eval mode
        scores = self.model(user_features, item_features)
        assert scores.shape == (batch_size,)

    def test_model_train_mode(self):
        """Test model behavior in training mode"""
        self.model.train()
        
        batch_size = 4
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Should work in training mode
        scores = self.model(user_features, item_features)
        assert scores.shape == (batch_size,)

    def test_invalid_input_shapes(self):
        """Test model behavior with invalid input shapes"""
        # Wrong user feature dimension
        user_features = torch.randn(4, 50)  # Should be 64
        item_features = torch.randn(4, TestConfig.ITEM_FEATURE_DIM)
        
        with pytest.raises(RuntimeError):
            self.model(user_features, item_features)

    def test_empty_batch(self):
        """Test model behavior with empty batch"""
        user_features = torch.empty(0, TestConfig.USER_FEATURE_DIM)
        item_features = torch.empty(0, TestConfig.ITEM_FEATURE_DIM)
        
        scores = self.model(user_features, item_features)
        assert scores.shape == (0,)

    def test_deterministic_output(self):
        """Test that model produces deterministic outputs with same inputs"""
        batch_size = 3
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        # Set model to eval mode for deterministic behavior
        self.model.eval()
        
        scores1 = self.model(user_features, item_features)
        scores2 = self.model(user_features, item_features)
        
        assert_tensors_close(scores1, scores2)

    def test_embedding_dimension_consistency(self):
        """Test that user and item embeddings have consistent dimensions"""
        batch_size = 5
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        user_emb = self.model.encode_user(user_features)
        item_emb = self.model.encode_item(item_features)
        
        assert user_emb.shape == item_emb.shape
        assert user_emb.shape[1] == TestConfig.EMBEDDING_DIM


class TestTwoTowerModelEdgeCases:
    """Test edge cases for TwoTowerModel"""

    def test_extreme_feature_values(self):
        """Test model with extreme feature values"""
        model = TwoTowerModel(
            user_feature_dim=64,
            item_feature_dim=32,
            embedding_dim=128,
        )
        
        # Very large values
        user_features = torch.full((2, 64), 1e6)
        item_features = torch.full((2, 32), 1e6)
        
        scores = model(user_features, item_features)
        assert torch.isfinite(scores).all()
        
        # Very small values
        user_features = torch.full((2, 64), 1e-6)
        item_features = torch.full((2, 32), 1e-6)
        
        scores = model(user_features, item_features)
        assert torch.isfinite(scores).all()

    def test_zero_features(self):
        """Test model with zero features"""
        model = TwoTowerModel(
            user_feature_dim=64,
            item_feature_dim=32,
            embedding_dim=128,
        )
        
        user_features = torch.zeros(3, 64)
        item_features = torch.zeros(3, 32)
        
        scores = model(user_features, item_features)
        assert torch.isfinite(scores).all()

    def test_single_feature_dimension(self):
        """Test model with minimal feature dimensions"""
        model = TwoTowerModel(
            user_feature_dim=1,
            item_feature_dim=1,
            embedding_dim=2,
            hidden_dims=[4],
        )
        
        user_features = torch.randn(2, 1)
        item_features = torch.randn(2, 1)
        
        scores = model(user_features, item_features)
        assert scores.shape == (2,)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])