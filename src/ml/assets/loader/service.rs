//! Netflix-grade model loader with retry-with-backoff, fallback to stale model,
//! circuit breaker on DB calls, and analytics integration.
//!
//! # Resilience Patterns
//! - **Retry**: Exponential backoff on transient load failures
//! - **Fallback**: Returns stale cached model if reload fails
//! - **Circuit Breaker**: DB queries go through resilient pool
//! - **Analytics**: Model load latency, cache hit/miss, reload events

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, Context};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::circuit_breaker::observer::ResilienceObserver;
use crate::config::MlConfig;
use crate::db::ModelRegistry;
use crate::db::ModelRepository;
use crate::error::ModelError;
use crate::ml::inference::onnx::service::OnnxInferenceEngine;

/// Metadata about a cached model for staleness tracking.
struct CachedModel {
    engine: Arc<OnnxInferenceEngine>,
    loaded_at: Instant,
}

/// Netflix-grade model loader with retry, fallback, and analytics.
pub struct ModelLoader {
    models: Arc<RwLock<HashMap<String, CachedModel>>>,
    model_dir: PathBuf,
    model_repo: Arc<ModelRepository>,
    config: MlConfig,
    observer: Arc<dyn ResilienceObserver>,
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

impl ModelLoader {
    pub fn new(
        model_dir: impl AsRef<Path>,
        model_repo: Arc<ModelRepository>,
        config: MlConfig,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
    ) -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            model_dir: model_dir.as_ref().to_path_buf(),
            model_repo,
            config,
            observer,
            analytics,
        }
    }

    /// Load all deployed ONNX models from database with retry.
    pub async fn load_all_models(&self) -> Result<usize> {
        info!("Loading all deployed ONNX models...");

        let deployed_models = self.model_repo.get_deployed_onnx_models().await?;
        let mut count = 0;

        for model_entry in deployed_models {
            match self.load_model_with_retry(&model_entry).await {
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
                        "Failed to load ONNX model after retries"
                    );
                }
            }
        }

        // Analytics: record load event
        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time("ml.model_loader.load_all", 0);
            analytics.increment_throughput("ml.model_loader.load_all");
        }

        info!(count = count, "ONNX models loaded");
        Ok(count)
    }

    /// Load a single model with exponential backoff retry.
    async fn load_model_with_retry(
        &self,
        model_entry: &ModelRegistry,
    ) -> Result<Arc<OnnxInferenceEngine>, ModelError> {
        let max_retries = self.config.model_load_max_retries;
        let base_backoff = self.config.model_load_base_backoff;
        let max_backoff = self.config.model_load_max_backoff;

        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.load_model(model_entry).await {
                Ok(engine) => return Ok(engine),
                Err(e) => {
                    if attempt < max_retries {
                        let backoff = std::cmp::min(
                            base_backoff * 2u32.pow(attempt as u32),
                            max_backoff,
                        );
                        warn!(
                            model = %model_entry.model_name,
                            attempt = attempt + 1,
                            max_retries = max_retries,
                            backoff_ms = backoff.as_millis() as u64,
                            error = %e,
                            "Model load failed, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                    last_error = Some(e);
                }
            }
        }

        // Fallback: try to return stale cached version
        let model_key = format!("{}:{}", model_entry.model_name, model_entry.version);
        if self.config.fallback_to_stale_model {
            if let Some(stale) = self.get_stale_model(&model_key).await {
                warn!(
                    model = %model_key,
                    "Returning stale model after load failure"
                );
                if let Some(ref analytics) = self.analytics {
                    analytics.increment_error("ml.model_loader.fallback_to_stale");
                }
                return Ok(stale);
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ModelError::LoadFailed(format!("exhausted {} retries for {}", max_retries, model_entry.model_name))
        }))
    }

    /// Load a single ONNX model (one attempt, no retry).
    async fn load_model(
        &self,
        model_entry: &ModelRegistry,
    ) -> Result<Arc<OnnxInferenceEngine>, ModelError> {
        let start = Instant::now();
        let metric_key = format!("ml.model_loader.load.{}", model_entry.model_name);

        let model_path = if let Some(onnx_path) = &model_entry.onnx_model_path {
            self.model_dir.join(onnx_path)
        } else {
            return Err(ModelError::NotFound(format!(
                "ONNX model path not set for {}",
                model_entry.model_name
            )));
        };

        if !model_path.exists() {
            return Err(ModelError::NotFound(format!(
                "ONNX model file not found: {:?}",
                model_path
            )));
        }

        let model_key = format!("{}:{}", model_entry.model_name, model_entry.version);
        let engine = OnnxInferenceEngine::new(
            &model_path,
            model_key.clone(),
            &self.config,
            self.observer.clone(),
            self.analytics.clone(),
        )?;

        let engine = Arc::new(engine);
        let latency = start.elapsed();

        // Analytics
        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time(&metric_key, latency.as_millis() as u64);
            analytics.increment_throughput(&metric_key);
        }

        // Cache in memory
        self.models.write().await.insert(model_key, CachedModel {
            engine: engine.clone(),
            loaded_at: Instant::now(),
        });

        Ok(engine)
    }

    /// Get a stale model from cache if within max stale age.
    async fn get_stale_model(&self, model_key: &str) -> Option<Arc<OnnxInferenceEngine>> {
        let models = self.models.read().await;
        if let Some(cached) = models.get(model_key) {
            if cached.loaded_at.elapsed() <= self.config.fallback_max_stale_age {
                return Some(cached.engine.clone());
            }
        }
        None
    }

    /// Get a loaded model by name (latest deployed version).
    /// This is the primary method used by pipeline stages.
    pub async fn get_model(
        &self,
        model_name: &str,
    ) -> Result<Arc<OnnxInferenceEngine>> {
        self.get_latest_model(model_name).await
    }

    /// Get a loaded model by name and specific version.
    pub async fn get_model_version(
        &self,
        model_name: &str,
        version: &str,
    ) -> Result<Arc<OnnxInferenceEngine>> {
        let model_key = format!("{}:{}", model_name, version);

        // Analytics: track cache hit/miss
        let metric_key = "ml.model_loader.cache";

        // Try to get from cache
        {
            let models = self.models.read().await;
            if let Some(cached) = models.get(&model_key) {
                if let Some(ref analytics) = self.analytics {
                    analytics.increment_throughput(&format!("{}.hit", metric_key));
                }
                return Ok(cached.engine.clone());
            }
        }

        if let Some(ref analytics) = self.analytics {
            analytics.increment_throughput(&format!("{}.miss", metric_key));
        }

        // Load from database if not cached
        let model_entry = self.model_repo
            .get_onnx_model(model_name, version)
            .await?
            .context(format!("Model not found: {}", model_key))?;

        let engine: Arc<OnnxInferenceEngine> = self.load_model_with_retry(&model_entry).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(engine)
    }

    /// Get a loaded model by name (latest deployed version).
    pub async fn get_latest_model(
        &self,
        model_name: &str,
    ) -> Result<Arc<OnnxInferenceEngine>> {
        let model_entry = self.model_repo
            .get_latest_deployed(model_name)
            .await?
            .context(format!("No deployed model found: {}", model_name))?;

        let model_key = format!("{}:{}", model_entry.model_name, model_entry.version);

        // Try to get from cache
        {
            let models = self.models.read().await;
            if let Some(cached) = models.get(&model_key) {
                return Ok(cached.engine.clone());
            }
        }

        let engine: Arc<OnnxInferenceEngine> = self.load_model_with_retry(&model_entry).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(engine)
    }

    /// Reload all models (hot-reload) with fallback to stale on failure.
    pub async fn reload_all(&self) -> Result<usize> {
        info!("Reloading all ONNX models...");

        // Keep stale models around for fallback
        let old_count = self.models.read().await.len();

        // Reload from database
        let count = self.load_all_models().await?;

        if let Some(ref analytics) = self.analytics {
            analytics.increment_throughput("ml.model_loader.reload");
        }

        info!(old_count = old_count, new_count = count, "Models reloaded");
        Ok(count)
    }

    /// Get count of loaded models.
    pub async fn loaded_count(&self) -> usize {
        self.models.read().await.len()
    }
}
