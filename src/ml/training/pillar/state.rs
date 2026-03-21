//! Training state management for the Sovereign Training Pillar.
//! Handles atomic weight swaps, disk persistence, and in-memory model telemetry.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use candle_core::{Device, Tensor};

/// Real-time metrics for the Sovereign Training Pillar.
#[derive(Debug, Clone, Default)]
pub struct TrainingMetrics {
    pub current_loss: f32,
    pub epochs_completed: u32,
    pub batches_trained: u64,
}

/// In-memory training state with atomic weight swap and disk persistence.
pub struct TrainingState {
    /// Live references to student head weights (Model ID -> (Tensor name -> Weight data))
    pub active_weights: Arc<RwLock<HashMap<String, HashMap<String, Tensor>>>>,
    /// Metadata about the current learning progress
    pub metrics: Arc<RwLock<TrainingMetrics>>,
    /// Path to the local model storage
    pub storage_path: PathBuf,
}

impl TrainingState {
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        Self {
            active_weights: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(TrainingMetrics::default())),
            storage_path: storage_path.as_ref().to_path_buf(),
        }
    }

    /// Save the current weights to a high-integrity storage on disk.
    pub async fn save_checkpoint(&self, model_id: &str) -> anyhow::Result<()> {
        // Tightly scope the read guard to prevent blocking writers during disk I/O
        let weights = {
            let active = self.active_weights.read().await;
            match active.get(model_id) {
                Some(w) => w.clone(),
                None => return Ok(()),
            }
        }; // Read guard is dropped here!

        if weights.is_empty() {
            return Ok(());
        }

        let file_path = self.storage_path.join(format!("{}_head.safetensors", model_id));
        
        // 1. Ensure parent directory exists (Async I/O)
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 2. Execute Blocking I/O via spawn_blocking (Sync-to-Async Bridge)
        // This prevents the filesystem write from freezing the tokio executor.
        let mid = model_id.to_string();
        let path = file_path.clone();
        
        tokio::task::spawn_blocking(move || {
            info!(model = %mid, path = %path.display(), "Persisting student head weights to disk...");
            
            // Use Candle's safetensors utility to serialize the HashMap<String, Tensor>
            candle_core::safetensors::save(&weights, &path)
                .map_err(|e| anyhow::anyhow!("Safetensors save failed: {}", e))
        }).await.map_err(|e| anyhow::anyhow!("Join error during checkpoint save: {}", e))??;

        info!(model = %model_id, path = %file_path.display(), "Checkpoint saved successfully to sovereign storage");
        Ok(())
    }

    /// Load the latest weights from disk into memory using Candle.
    pub async fn load_checkpoint(&self, model_id: &str) -> anyhow::Result<bool> {
        let mut file_path = self.storage_path.join(format!("{}_head.safetensors", model_id));
        
        // Try fallback to the models/ directory if storage_path doesn't have it
        if !file_path.exists() {
            let fallback = PathBuf::from("models").join(format!("{}_head.safetensors", model_id));
            if fallback.exists() {
                file_path = fallback;
            } else {
                debug!(model = %model_id, "No checkpoint found at {} or models/", self.storage_path.display());
                return Ok(false);
            }
        }

        info!(model = %model_id, path = %file_path.display(), "Loading student head weights via Candle...");
        
        // Use Candle's built-in safetensors loader (runs on CPU for initial load)
        let tensors = candle_core::safetensors::load(&file_path, &Device::Cpu)?;
        
        let mut active = self.active_weights.write().await;
        active.insert(model_id.to_string(), tensors);

        info!(model = %model_id, "Student head weights loaded into memory successfully");
        Ok(true)
    }

    /// Perform an atomic swap of model weights in memory and trigger a background save.
    pub async fn commit_weights(&self, model_id: &str, weights: HashMap<String, Tensor>) {
        {
            let mut active = self.active_weights.write().await;
            active.insert(model_id.to_string(), weights);
        }
        
        let storage = self.storage_path.clone();
        let weights_clone = self.active_weights.clone();
        let mid = model_id.to_string();
        
        tokio::spawn(async move {
            let state = TrainingState {
                active_weights: weights_clone,
                metrics: Arc::new(RwLock::new(TrainingMetrics::default())),
                storage_path: storage,
            };
            if let Err(e) = state.save_checkpoint(&mid).await {
                error!(error = %e, model = %mid, "Failed to persist checkpoint");
            }
        });

        debug!(model = %model_id, "Atomic weight swap committed");
    }

    pub async fn record_loss(&self, loss: f32) {
        let mut metrics = self.metrics.write().await;
        metrics.current_loss = loss;
        metrics.batches_trained += 1;
    }

    pub async fn increment_epoch(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.epochs_completed += 1;
    }
}

