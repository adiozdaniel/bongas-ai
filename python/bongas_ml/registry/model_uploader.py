"""
Upload ONNX models to PostgreSQL model_registry table
"""

import asyncio
import asyncpg
import json
from pathlib import Path
from datetime import datetime
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def upload_onnx_model(
    db_url: str,
    model_name: str,
    version: str,
    onnx_path: str,
    user_feature_dim: int,
    item_feature_dim: int,
    opset_version: int,
):
    """
    Upload ONNX model metadata to PostgreSQL
    """
    conn = await asyncpg.connect(db_url)

    try:
        # Insert or update model registry
        await conn.execute(
            """
            INSERT INTO model_registry (
                model_name, version, model_format, onnx_model_path,
                onnx_opset_version, onnx_input_shapes, onnx_output_names,
                onnx_runtime_provider, status, deployed_at
            )
            VALUES ($1, $2, 'onnx', $3, $4, $5, $6, 'cpu', 'deployed', NOW())
            ON CONFLICT (model_name, version) DO UPDATE SET
                onnx_model_path = EXCLUDED.onnx_model_path,
                onnx_opset_version = EXCLUDED.onnx_opset_version,
                onnx_input_shapes = EXCLUDED.onnx_input_shapes,
                onnx_output_names = EXCLUDED.onnx_output_names,
                deployed_at = NOW()
            """,
            model_name,
            version,
            onnx_path,
            opset_version,
            json.dumps({
                'user_features': [-1, user_feature_dim],
                'item_features': [-1, item_feature_dim],
            }),
            json.dumps(['scores']),
        )

        logger.info(f"Uploaded {model_name} v{version} to model registry")

    finally:
        await conn.close()


if __name__ == "__main__":
    # Example usage
    asyncio.run(upload_onnx_model(
        db_url="postgresql://postgres:password@localhost:5432/baze_catalog",
        model_name="two_tower",
        version="v1.0",
        onnx_path="models/onnx/two_tower_v1.onnx",
        user_feature_dim=64,
        item_feature_dim=32,
        opset_version=17,
    ))
