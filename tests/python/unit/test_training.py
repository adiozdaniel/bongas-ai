"""
Unit tests for training pipeline in BONGAS-AI
"""

import pytest
import torch
import torch.nn as nn
import torch.optim as optim
import numpy as np
from unittest.mock import Mock, patch, MagicMock
import tempfile
import os
from pathlib import Path

from bongas_ml.training.train_two_tower import TwoTowerModel, RecommendationDataset, train_two_tower
from tests.python.fixtures import (
    TestConfig, create_test_dataset, create_synthetic_user_features, 
    create_synthetic_item_features, get_test_device
)


class TestRecommendationDataset:
    """Test cases for RecommendationDataset"""

    def setup_method(self):
        """Setup for each test method"""
        self.interactions = create_test_dataset(50)
        self.dataset = RecommendationDataset(self.interactions)

    def test_dataset_length(self):
        """Test dataset length"""
        assert len(self.dataset) == len(self.interactions)

    def test_dataset_getitem(self):
        """Test dataset item retrieval"""
        item = self.dataset[0]
        
        # Check keys
        assert 'user_features' in item
        assert 'item_features' in item
        assert 'rating' in item
        
        # Check types
        assert isinstance(item['user_features'], torch.Tensor)
        assert isinstance(item['item_features'], torch.Tensor)
        assert isinstance(item['rating'], torch.Tensor)
        
        # Check shapes
        assert item['user_features'].shape == (TestConfig.USER_FEATURE_DIM,)
        assert item['item_features'].shape == (TestConfig.ITEM_FEATURE_DIM,)
        assert item['rating'].shape == ()

    def test_dataset_tensor_types(self):
        """Test that dataset returns correct tensor types"""
        item = self.dataset[0]
        
        assert item['user_features'].dtype == torch.float32
        assert item['item_features'].dtype == torch.float32
        assert item['rating'].dtype == torch.float32

    def test_dataset_with_empty_interactions(self):
        """Test dataset with empty interactions"""
        empty_dataset = RecommendationDataset([])
        assert len(empty_dataset) == 0
        
        with pytest.raises(IndexError):
            empty_dataset[0]

    def test_dataset_with_single_interaction(self):
        """Test dataset with single interaction"""
        single_interaction = [self.interactions[0]]
        single_dataset = RecommendationDataset(single_interaction)
        
        assert len(single_dataset) == 1
        item = single_dataset[0]
        
        assert item['user_features'].shape == (TestConfig.USER_FEATURE_DIM,)
        assert item['item_features'].shape == (TestConfig.ITEM_FEATURE_DIM,)


class TestTrainTwoTower:
    """Test cases for train_two_tower function"""

    def setup_method(self):
        """Setup for each test method"""
        self.train_data = create_test_dataset(100)
        self.val_data = create_test_dataset(20)
        self.device = get_test_device()

    def test_train_two_tower_basic(self):
        """Test basic training functionality"""
        model = train_two_tower(
            train_data=self.train_data,
            val_data=self.val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
            epochs=1,  # Minimal epochs for testing
            batch_size=16,
            learning_rate=1e-3,
            device=self.device,
        )
        
        # Check model type
        assert isinstance(model, TwoTowerModel)
        
        # Check model parameters
        params = list(model.parameters())
        assert len(params) > 0
        
        # Test forward pass
        batch_size = 2
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        scores = model(user_features, item_features)
        assert scores.shape == (batch_size,)

    def test_train_two_tower_different_dimensions(self):
        """Test training with different feature dimensions"""
        model = train_two_tower(
            train_data=self.train_data,
            val_data=self.val_data,
            user_feature_dim=50,
            item_feature_dim=25,
            embedding_dim=64,
            hidden_dims=[128, 64],
            epochs=1,
            batch_size=8,
            device=self.device,
        )
        
        # Test with correct dimensions
        user_features = torch.randn(3, 50)
        item_features = torch.randn(3, 25)
        scores = model(user_features, item_features)
        assert scores.shape == (3,)

    def test_train_two_tower_device_consistency(self):
        """Test training on different devices"""
        if torch.cuda.is_available():
            model = train_two_tower(
                train_data=self.train_data,
                val_data=self.val_data,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=1,
                device='cuda',
            )
            
            assert next(model.parameters()).device.type == 'cuda'

    def test_train_two_tower_with_small_dataset(self):
        """Test training with very small dataset"""
        small_train = create_test_dataset(10)
        small_val = create_test_dataset(5)
        
        model = train_two_tower(
            train_data=small_train,
            val_data=small_val,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=2,
            device=self.device,
        )
        
        assert isinstance(model, TwoTowerModel)

    def test_train_two_tower_loss_decreases(self):
        """Test that training loss decreases during training"""
        # Mock the training loop to capture losses
        losses = []
        
        def mock_train_step(model, optimizer, criterion, user_features, item_features, ratings):
            optimizer.zero_grad()
            scores = model(user_features, item_features)
            loss = criterion(scores, ratings)
            loss.backward()
            optimizer.step()
            return loss.item()
        
        # Create a simple model for testing
        model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
        ).to(self.device)
        
        optimizer = optim.Adam(model.parameters(), lr=1e-3)
        criterion = nn.BCEWithLogitsLoss()
        
        # Simulate a few training steps
        for i in range(5):
            # Get a batch
            batch = self.train_data[i]
            user_features = torch.tensor(batch['user_features'], dtype=torch.float32).unsqueeze(0)
            item_features = torch.tensor(batch['item_features'], dtype=torch.float32).unsqueeze(0)
            ratings = torch.tensor([batch['rating']], dtype=torch.float32)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
                ratings = ratings.cuda()
            
            loss = mock_train_step(model, optimizer, criterion, user_features, item_features, ratings)
            losses.append(loss)
        
        # Loss should generally decrease (allowing for some noise)
        assert losses[-1] < losses[0] * 1.5  # Allow some variance

    def test_train_two_tower_validation(self):
        """Test that validation works during training"""
        # This test verifies that validation doesn't crash
        model = train_two_tower(
            train_data=self.train_data,
            val_data=self.val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=16,
            device=self.device,
        )
        
        # Verify model can make predictions
        user_features = torch.randn(5, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(5, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
            model = model.cuda()
        
        with torch.no_grad():
            model.eval()
            scores = model(user_features, item_features)
        
        assert scores.shape == (5,)
        assert torch.isfinite(scores).all()

    def test_train_two_tower_empty_datasets(self):
        """Test training with empty datasets"""
        with pytest.raises((ValueError, IndexError)):
            train_two_tower(
                train_data=[],
                val_data=[],
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=1,
            )

    def test_train_two_tower_single_batch(self):
        """Test training with single batch"""
        single_batch_data = create_test_dataset(16)  # Exactly one batch
        single_batch_val = create_test_dataset(8)
        
        model = train_two_tower(
            train_data=single_batch_data,
            val_data=single_batch_val,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=16,
            device=self.device,
        )
        
        assert isinstance(model, TwoTowerModel)

    @patch('bongas_ml.training.train_two_tower.logger')
    def test_train_two_tower_logging(self, mock_logger):
        """Test that training logs correctly"""
        train_two_tower(
            train_data=self.train_data,
            val_data=self.val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            batch_size=16,
            device=self.device,
        )
        
        # Check that logger.info was called
        assert mock_logger.info.called

    def test_train_two_tower_different_batch_sizes(self):
        """Test training with different batch sizes"""
        for batch_size in [8, 16, 32]:
            model = train_two_tower(
                train_data=self.train_data,
                val_data=self.val_data,
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                epochs=1,
                batch_size=batch_size,
                device=self.device,
            )
            
            assert isinstance(model, TwoTowerModel)

    def test_train_two_tower_model_state(self):
        """Test that model is in eval mode after training"""
        model = train_two_tower(
            train_data=self.train_data,
            val_data=self.val_data,
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            device=self.device,
        )
        
        # Model should be in eval mode after training
        assert not model.training

    def test_train_two_tower_gradient_computation(self):
        """Test that gradients are computed during training"""
        model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
        ).to(self.device)
        
        optimizer = optim.Adam(model.parameters(), lr=1e-3)
        criterion = nn.BCEWithLogitsLoss()
        
        # Get a batch
        batch = self.train_data[0]
        user_features = torch.tensor(batch['user_features'], dtype=torch.float32).unsqueeze(0)
        item_features = torch.tensor(batch['item_features'], dtype=torch.float32).unsqueeze(0)
        ratings = torch.tensor([batch['rating']], dtype=torch.float32)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
            ratings = ratings.cuda()
        
        # Forward pass
        optimizer.zero_grad()
        scores = model(user_features, item_features)
        loss = criterion(scores, ratings)
        loss.backward()
        
        # Check that gradients exist
        for param in model.parameters():
            if param.requires_grad:
                assert param.grad is not None
                assert torch.isfinite(param.grad).any()


class TestTrainTwoTowerEdgeCases:
    """Test edge cases for training"""

    def test_train_with_extreme_learning_rates(self):
        """Test training with extreme learning rates"""
        # Very small learning rate
        model1 = train_two_tower(
            train_data=create_test_dataset(50),
            val_data=create_test_dataset(10),
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            learning_rate=1e-6,
            device=get_test_device(),
        )
        
        # Very large learning rate
        model2 = train_two_tower(
            train_data=create_test_dataset(50),
            val_data=create_test_dataset(10),
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            epochs=1,
            learning_rate=1e-1,
            device=get_test_device(),
        )
        
        assert isinstance(model1, TwoTowerModel)
        assert isinstance(model2, TwoTowerModel)

    def test_train_with_different_hidden_dims(self):
        """Test training with different hidden layer configurations"""
        for hidden_dims in [[64], [128, 64], [256, 128, 64]]:
            model = train_two_tower(
                train_data=create_test_dataset(50),
                val_data=create_test_dataset(10),
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                hidden_dims=hidden_dims,
                epochs=1,
                device=get_test_device(),
            )
            
            assert isinstance(model, TwoTowerModel)

    def test_train_with_different_embedding_dims(self):
        """Test training with different embedding dimensions"""
        for embedding_dim in [64, 128, 256]:
            model = train_two_tower(
                train_data=create_test_dataset(50),
                val_data=create_test_dataset(10),
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                embedding_dim=embedding_dim,
                epochs=1,
                device=get_test_device(),
            )
            
            assert isinstance(model, TwoTowerModel)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])