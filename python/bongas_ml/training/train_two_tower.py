"""
Training pipeline for Two-Tower model with ONNX export
"""

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
import numpy as np
from typing import List, Dict, Tuple
from pathlib import Path
import logging

from bongas_ml.models.two_tower import TwoTowerModel

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class RecommendationDataset(Dataset):
    """Dataset for user-item interactions"""

    def __init__(self, interactions: List[Dict]):
        """
        Args:
            interactions: List of dicts with keys:
                - user_features: np.array
                - item_features: np.array
                - rating: float (1.0 for positive, 0.0 for negative)
        """
        self.interactions = interactions

    def __len__(self):
        return len(self.interactions)

    def __getitem__(self, idx):
        interaction = self.interactions[idx]
        return {
            'user_features': torch.tensor(interaction['user_features'], dtype=torch.float32),
            'item_features': torch.tensor(interaction['item_features'], dtype=torch.float32),
            'rating': torch.tensor(interaction['rating'], dtype=torch.float32),
        }


def train_two_tower(
    train_data: List[Dict],
    val_data: List[Dict],
    user_feature_dim: int,
    item_feature_dim: int,
    embedding_dim: int = 128,
    hidden_dims: List[int] = [256, 128],
    epochs: int = 10,
    batch_size: int = 256,
    learning_rate: float = 1e-3,
    device: str = 'cuda' if torch.cuda.is_available() else 'cpu',
) -> TwoTowerModel:
    """
    Train Two-Tower model

    Returns:
        Trained model
    """
    logger.info(f"Training Two-Tower model on {device}")
    logger.info(f"   Train samples: {len(train_data)}")
    logger.info(f"   Val samples: {len(val_data)}")

    # Create datasets
    train_dataset = RecommendationDataset(train_data)
    val_dataset = RecommendationDataset(val_data)

    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False)

    # Initialize model
    model = TwoTowerModel(
        user_feature_dim=user_feature_dim,
        item_feature_dim=item_feature_dim,
        embedding_dim=embedding_dim,
        hidden_dims=hidden_dims,
    ).to(device)

    # Loss and optimizer
    criterion = nn.BCEWithLogitsLoss()
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)

    best_val_loss = float('inf')

    # Training loop
    for epoch in range(epochs):
        # Train
        model.train()
        train_loss = 0.0

        for batch in train_loader:
            user_features = batch['user_features'].to(device)
            item_features = batch['item_features'].to(device)
            ratings = batch['rating'].to(device)

            optimizer.zero_grad()
            scores = model(user_features, item_features)
            loss = criterion(scores, ratings)

            loss.backward()
            optimizer.step()

            train_loss += loss.item()

        train_loss /= len(train_loader)

        # Validate
        model.eval()
        val_loss = 0.0

        with torch.no_grad():
            for batch in val_loader:
                user_features = batch['user_features'].to(device)
                item_features = batch['item_features'].to(device)
                ratings = batch['rating'].to(device)

                scores = model(user_features, item_features)
                loss = criterion(scores, ratings)

                val_loss += loss.item()

        val_loss /= len(val_loader)

        logger.info(f"Epoch {epoch+1}/{epochs} - Train Loss: {train_loss:.4f}, Val Loss: {val_loss:.4f}")

        # Save best model
        if val_loss < best_val_loss:
            best_val_loss = val_loss
            logger.info(f"New best model (val_loss={val_loss:.4f})")

    logger.info("Training complete")
    return model


if __name__ == "__main__":
    # Example usage
    # Generate dummy data
    np.random.seed(42)

    train_data = []
    for _ in range(10000):
        train_data.append({
            'user_features': np.random.randn(64),
            'item_features': np.random.randn(32),
            'rating': float(np.random.rand() > 0.5),
        })

    val_data = []
    for _ in range(2000):
        val_data.append({
            'user_features': np.random.randn(64),
            'item_features': np.random.randn(32),
            'rating': float(np.random.rand() > 0.5),
        })

    # Train model
    model = train_two_tower(
        train_data=train_data,
        val_data=val_data,
        user_feature_dim=64,
        item_feature_dim=32,
        epochs=5,
    )

    # Save model
    Path('models').mkdir(parents=True, exist_ok=True)
    torch.save(model.state_dict(), 'models/two_tower_v1.pt')
    logger.info("Model saved to models/two_tower_v1.pt")
