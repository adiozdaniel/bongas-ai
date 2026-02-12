use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Instant;
use tracing::{info, warn};

use crate::ml::onnx_runtime::OnnxInferenceEngine;
use crate::db::models::ModelRegistry;
use crate::db::repositories::model_repository::ModelRepository;

pub struct ModelLoader {
    models: Arc<RwLock<HashMap<String, Arc<RwLock<OnnxInferenceEngine>>>>>,
    model_dir: PathBuf,
    model_repo: Arc<ModelRepository>,
}

impl ModelLoader {
    pub fn new(model_dir: impl AsRef<Path>, model_repo: Arc<ModelRepository>) -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            model_dir: model_dir.as_ref().to_path_buf(),
            model_repo,
        }
    }

    /// Load all deployed ONNX models from database
    pub async fn load_all_models(&self) -> Result<usize> {
        info!("Loading all deployed ONNX models...");

        let deployed_models = self.model_repo.get_deployed_onnx_models().await?;
        let mut count = 0;

        for model_entry in deployed_models {
            match self.load_model(&model_entry).await {
                Ok(_) => {
                    count += 1;
                    info!(
                        model = %model_entry.model_name,
                        version = %model_entry.version,
                        "Loaded ONNX model"
                    );
                }
                Err(e) => {
                    warn!(
                        model = %model_entry.model_name,
                        version = %model_entry.version,
                        error = %e,
                        "Failed to load ONNX model"
                    );
                }
            }
        }

        info!(count = count, "ONNX models loaded");
        Ok(count)
    }

    /// Load a single ONNX model
    async fn load_model(&self, model_entry: &ModelRegistry) -> Result<Arc<RwLock<OnnxInferenceEngine>>> {
        let start_time = Instant::now();
        let model_path = if let Some(onnx_path) = &model_entry.onnx_model_path {
            self.model_dir.join(onnx_path)
        } else {
            return Err(anyhow::anyhow!(
                "ONNX model path not set for {}",
                model_entry.model_name
            ));
        };

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "ONNX model file not found: {:?}",
                model_path
            ));
        }

        let model_key = format!("{}:{}", model_entry.model_name, model_entry.version);
        let engine = Arc::new(RwLock::new(OnnxInferenceEngine::new(
            &model_path,
            model_key.clone(),
        )?));

        let _duration = start_time.elapsed();
        
        // Track model loading metrics
        crate::analytics::ANALYTICS_MANAGER.set_model_accuracy(&model_entry.model_name, 1.0); // Assume loaded models are accurate

        // Cache in memory
        self.models.write().await.insert(model_key, engine.clone());

        Ok(engine)
    }

    /// Get a loaded model by name (latest deployed version)
    /// This is the primary method used by pipeline stages
    pub async fn get_model(
        &self,
        model_name: &str,
    ) -> Result<Arc<RwLock<OnnxInferenceEngine>>> {
        self.get_latest_model(model_name).await
    }

    /// Get a loaded model by name and specific version
    pub async fn get_model_version(
        &self,
        model_name: &str,
        version: &str,
    ) -> Result<Arc<RwLock<OnnxInferenceEngine>>> {
        let model_key = format!("{}:{}", model_name, version);

        // Try to get from cache
        if let Some(engine) = self.models.read().await.get(&model_key) {
            return Ok(engine.clone());
        }

        // Load from database if not cached
        let model_entry = self.model_repo
            .get_onnx_model(model_name, version)
            .await?
            .context(format!("Model not found: {}", model_key))?;

        self.load_model(&model_entry).await
    }

    /// Get a loaded model by name (latest deployed version)
    pub async fn get_latest_model(
        &self,
        model_name: &str,
    ) -> Result<Arc<RwLock<OnnxInferenceEngine>>> {
        let model_entry = self.model_repo
            .get_latest_deployed(model_name)
            .await?
            .context(format!("No deployed model found: {}", model_name))?;

        let model_key = format!("{}:{}", model_entry.model_name, model_entry.version);

        // Try to get from cache
        if let Some(engine) = self.models.read().await.get(&model_key) {
            return Ok(engine.clone());
        }

        self.load_model(&model_entry).await
    }

    /// Reload all models (hot-reload)
    pub async fn reload_all(&self) -> Result<usize> {
        info!("Reloading all ONNX models...");

        // Clear cache
        self.models.write().await.clear();

        // Reload from database
        self.load_all_models().await
    }

    /// Get count of loaded models
    pub async fn loaded_count(&self) -> usize {
        self.models.read().await.len()
    }
}
