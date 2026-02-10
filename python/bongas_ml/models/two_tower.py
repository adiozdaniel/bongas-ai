"""
Two-Tower Neural Network for User-Item Recommendations

Architecture:
- User Tower: Encodes user features -> embedding
- Item Tower: Encodes item features -> embedding
- Similarity: Dot product between normalized embeddings

This code stays in development environment - only ONNX models are shipped.
"""

import torch
import torch.nn as nn
import numpy as np
from typing import List, Tuple, Optional


class TwoTowerModel(nn.Module):
    """Dual-encoder architecture for recommendation"""

    def __init__(
        self,
        user_feature_dim: int,
        item_feature_dim: int,
        embedding_dim: int = 128,
        hidden_dims: List[int] = [256, 128],
        dropout: float = 0.2,
    ):
        super().__init__()

        # User tower
        user_layers = []
        prev_dim = user_feature_dim
        for hidden_dim in hidden_dims:
            user_layers.extend([
                nn.Linear(prev_dim, hidden_dim),
                nn.ReLU(),
                nn.BatchNorm1d(hidden_dim),
                nn.Dropout(dropout),
            ])
            prev_dim = hidden_dim
        user_layers.append(nn.Linear(prev_dim, embedding_dim))
        self.user_tower = nn.Sequential(*user_layers)

        # Item tower
        item_layers = []
        prev_dim = item_feature_dim
        for hidden_dim in hidden_dims:
            item_layers.extend([
                nn.Linear(prev_dim, hidden_dim),
                nn.ReLU(),
                nn.BatchNorm1d(hidden_dim),
                nn.Dropout(dropout),
            ])
            prev_dim = hidden_dim
        item_layers.append(nn.Linear(prev_dim, embedding_dim))
        self.item_tower = nn.Sequential(*item_layers)

    def forward(
        self,
        user_features: torch.Tensor,
        item_features: torch.Tensor
    ) -> torch.Tensor:
        """
        Forward pass

        Args:
            user_features: (batch_size, user_feature_dim)
            item_features: (batch_size, item_feature_dim)

        Returns:
            scores: (batch_size,) similarity scores
        """
        # Encode
        user_emb = self.user_tower(user_features)  # (batch_size, embedding_dim)
        item_emb = self.item_tower(item_features)  # (batch_size, embedding_dim)

        # Normalize
        user_emb = nn.functional.normalize(user_emb, p=2, dim=1)
        item_emb = nn.functional.normalize(item_emb, p=2, dim=1)

        # Dot product similarity
        scores = (user_emb * item_emb).sum(dim=1)

        return scores

    def encode_user(self, user_features: torch.Tensor) -> torch.Tensor:
        """Encode user features to embedding"""
        user_emb = self.user_tower(user_features)
        return nn.functional.normalize(user_emb, p=2, dim=1)

    def encode_item(self, item_features: torch.Tensor) -> torch.Tensor:
        """Encode item features to embedding"""
        item_emb = self.item_tower(item_features)
        return nn.functional.normalize(item_emb, p=2, dim=1)
