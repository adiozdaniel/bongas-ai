use std::sync::Arc;
use crate::ml::inference::candle::service::CandleInferenceEngine;
use crate::ml::inference::embeddings::service::EmbeddingManager;
use crate::ml::inference::features::service::FeatureStore;

/// 🎯 THE STAGE: High-performance real-time inference.
/// 
/// Consolidates all latency-sensitive ML operations into a single functional pillar.
pub struct InferencePillar {
    pub candle: Arc<CandleInferenceEngine>,
    pub embeddings: Arc<EmbeddingManager>,
    pub features: Arc<FeatureStore>,
}

impl InferencePillar {
    pub fn new(
        candle: Arc<CandleInferenceEngine>,
        embeddings: Arc<EmbeddingManager>,
        features: Arc<FeatureStore>,
    ) -> Self {
        Self {
            candle,
            embeddings,
            features,
        }
    }
}
