use std::sync::Arc;
use crate::ml::inference::onnx::service::OnnxInferenceEngine;
use crate::ml::inference::embeddings::service::EmbeddingManager;
use crate::ml::inference::features::service::FeatureStore;

/// 🎯 THE STAGE: High-performance real-time inference.
/// 
/// Consolidates all latency-sensitive ML operations into a single functional pillar.
pub struct InferencePillar {
    pub onnx: Arc<OnnxInferenceEngine>,
    pub embeddings: Arc<EmbeddingManager>,
    pub features: Arc<FeatureStore>,
}

impl InferencePillar {
    pub fn new(
        onnx: Arc<OnnxInferenceEngine>,
        embeddings: Arc<EmbeddingManager>,
        features: Arc<FeatureStore>,
    ) -> Self {
        Self {
            onnx,
            embeddings,
            features,
        }
    }
}
