use std::sync::Arc;
use crate::ml::assets::registry::service::VersionedModelRegistry;
use crate::ml::assets::loader::service::ModelLoader;

/// 📦 THE PULSE: ML Infrastructure & Model Lifecycle.
/// 
/// Manages model registration, versioning, canary routing, and hot-swapping.
pub struct AssetsPillar {
    pub registry: Arc<VersionedModelRegistry>,
    pub loader: Arc<ModelLoader>,
}

impl AssetsPillar {
    pub fn new(
        registry: Arc<VersionedModelRegistry>,
        loader: Arc<ModelLoader>,
    ) -> Self {
        Self {
            registry,
            loader,
        }
    }
}
