"""
Test fixtures and utilities for BONGAS-AI Python tests
"""

import torch
import numpy as np
import pandas as pd
from typing import Dict, List, Tuple, Any
import tempfile
import os
from pathlib import Path


class TestConfig:
    """Test configuration settings"""
    
    # Model dimensions
    USER_FEATURE_DIM = 64
    ITEM_FEATURE_DIM = 32
    EMBEDDING_DIM = 128
    HIDDEN_DIMS = [256, 128]
    
    # Training settings
    BATCH_SIZE = 32
    LEARNING_RATE = 1e-3
    EPOCHS = 2  # Small for testing
    
    # Test data settings
    NUM_USERS = 100
    NUM_ITEMS = 50
    NUM_INTERACTIONS = 1000
    
    # ONNX settings
    ONNX_OPSET_VERSION = 17


def create_synthetic_user_features(n_users: int = TestConfig.NUM_USERS, 
                                 feature_dim: int = TestConfig.USER_FEATURE_DIM) -> np.ndarray:
    """Create synthetic user features"""
    np.random.seed(42)
    return np.random.randn(n_users, feature_dim).astype(np.float32)


def create_synthetic_item_features(n_items: int = TestConfig.NUM_ITEMS,
                                 feature_dim: int = TestConfig.ITEM_FEATURE_DIM) -> np.ndarray:
    """Create synthetic item features"""
    np.random.seed(123)
    return np.random.randn(n_items, feature_dim).astype(np.float32)


def create_synthetic_interactions(n_interactions: int = TestConfig.NUM_INTERACTIONS,
                                n_users: int = TestConfig.NUM_USERS,
                                n_items: int = TestConfig.NUM_ITEMS) -> List[Dict]:
    """Create synthetic user-item interactions"""
    np.random.seed(456)
    
    interactions = []
    for _ in range(n_interactions):
        user_id = np.random.randint(0, n_users)
        item_id = np.random.randint(0, n_items)
        
        # Create features
        user_features = np.random.randn(TestConfig.USER_FEATURE_DIM).astype(np.float32)
        item_features = np.random.randn(TestConfig.ITEM_FEATURE_DIM).astype(np.float32)
        
        # Create rating (binary for simplicity)
        rating = float(np.random.rand() > 0.3)  # 70% positive
        
        interactions.append({
            'user_id': user_id,
            'item_id': item_id,
            'user_features': user_features,
            'item_features': item_features,
            'rating': rating,
            'timestamp': np.random.randint(1000000, 9999999)
        })
    
    return interactions


def create_test_dataset(n_samples: int = 100) -> List[Dict]:
    """Create a small test dataset for unit tests"""
    return create_synthetic_interactions(n_samples, 10, 5)


def get_test_device():
    """Get the best available device for testing"""
    return 'cuda' if torch.cuda.is_available() else 'cpu'


def create_temp_model_path(model_name: str = "test_model") -> str:
    """Create a temporary file path for model saving"""
    temp_dir = tempfile.mkdtemp()
    return os.path.join(temp_dir, f"{model_name}.pt")


def create_temp_onnx_path(model_name: str = "test_model") -> str:
    """Create a temporary file path for ONNX model saving"""
    temp_dir = tempfile.mkdtemp()
    return os.path.join(temp_dir, f"{model_name}.onnx")


class MockModel:
    """Mock model for testing purposes"""
    
    def __init__(self, user_dim: int = 64, item_dim: int = 32, embedding_dim: int = 128):
        self.user_dim = user_dim
        self.item_dim = item_dim
        self.embedding_dim = embedding_dim
        
    def forward(self, user_features, item_features):
        """Mock forward pass"""
        batch_size = user_features.shape[0]
        return torch.randn(batch_size)
    
    def state_dict(self):
        """Mock state dict"""
        return {
            'user_tower.0.weight': torch.randn(self.embedding_dim, self.user_dim),
            'item_tower.0.weight': torch.randn(self.embedding_dim, self.item_dim),
        }


def assert_tensors_close(tensor1: torch.Tensor, tensor2: torch.Tensor, 
                        atol: float = 1e-6, rtol: float = 1e-5):
    """Assert that two tensors are close"""
    assert torch.allclose(tensor1, tensor2, atol=atol, rtol=rtol), \
        f"Tensors not close: max diff = {torch.max(torch.abs(tensor1 - tensor2)).item()}"


def assert_shapes_match(tensor: torch.Tensor, expected_shape: Tuple[int, ...]):
    """Assert that tensor shape matches expected shape"""
    assert tensor.shape == expected_shape, \
        f"Shape mismatch: expected {expected_shape}, got {tensor.shape}"


def create_test_dataframe(n_rows: int = 100) -> pd.DataFrame:
    """Create a test pandas DataFrame"""
    np.random.seed(789)
    
    data = {
        'user_id': np.random.randint(0, 10, n_rows),
        'item_id': np.random.randint(0, 5, n_rows),
        'rating': np.random.uniform(0, 1, n_rows),
        'timestamp': np.random.randint(1000000, 9999999, n_rows),
        'user_age': np.random.randint(18, 65, n_rows),
        'item_category': np.random.choice(['A', 'B', 'C'], n_rows),
    }
    
    return pd.DataFrame(data)


def cleanup_temp_files():
    """Clean up temporary test files"""
    import shutil
    import glob
    
    # Clean up temporary directories
    temp_dirs = glob.glob('/tmp/tmp*')
    for temp_dir in temp_dirs:
        try:
            shutil.rmtree(temp_dir)
        except:
            pass