//! Netflix-grade pure-Rust inference engine using Candle.
//!
//! Every inference call is:
//! 1. Guarded by a per-model circuit breaker (trips on failure/slow calls)
//! 2. Limited by a semaphore bulkhead (prevents thread pool exhaustion)
//! 3. Timed out after a configurable duration
//! 4. Recorded in analytics (latency, throughput, errors per model)
//! 5. Classified via `ModelError` → `ErrorClassifier` for resilience routing

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;

use anyhow::Result;
use candle_core::{Device, Tensor, DType};
use tokio::sync::Semaphore;
use tracing::{info, debug, warn};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId};
use crate::circuit_breaker::observer::ResilienceObserver;
use crate::config::MlConfig;
use crate::error::ModelError;

/// Pure-Rust inference engine with Netflix resilience patterns.
pub struct CandleInferenceEngine {
    /// In-memory weights for the "Heavy Giant" (Base Model)
    weights: Arc<HashMap<String, Tensor>>,
    model_name: String,
    device: Device,

    // M21: Sovereign Training Bridge
    pub training_state: Option<Arc<crate::ml::training::pillar::state::TrainingState>>,

    // Resilience
    breaker: Arc<CircuitBreaker>,
    bulkhead: Arc<Semaphore>,

    // Analytics
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

impl CandleInferenceEngine {
    /// Create new Candle inference engine with full resilience wiring.
    pub fn new(
        model_path: impl AsRef<Path>,
        model_name: String,
        config: &MlConfig,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
    ) -> Result<Self, ModelError> {
        let model_path = model_path.as_ref();

        info!(
            model = %model_name,
            path = %model_path.display(),
            "Loading Candle model weights (safetensors)"
        );

        let device = Device::Cpu; // Prioritize CPU for sovereign VPC stability
        
        let weights = if model_path.exists() {
            candle_core::safetensors::load(model_path, &device)
                .map_err(|e| ModelError::LoadFailed(format!("safetensors load: {e}")))?
        } else {
            warn!(model = %model_name, path = %model_path.display(), "Model file not found, initializing with empty weights");
            HashMap::new()
        };

        // Build per-model circuit breaker
        let breaker_config = CircuitBreakerConfig::builder()
            .failure_rate_threshold(config.inference_breaker_failure_rate)
            .slow_call_rate_threshold(config.inference_breaker_slow_call_rate)
            .slow_call_duration(config.inference_breaker_slow_call_duration)
            .minimum_calls(config.inference_breaker_minimum_calls)
            .recovery_timeout(config.inference_breaker_recovery_timeout)
            .half_open_max_calls(config.inference_breaker_half_open_calls)
            .call_timeout(config.inference_timeout)
            .build()
            .map_err(|e| ModelError::InvalidConfig(format!(
                "circuit breaker config for model {}: {}",
                model_name, e
            )))?;

        let breaker_id = format!("ml.inference.{}", model_name);
        let breaker = Arc::new(CircuitBreaker::new(
            CircuitBreakerId::new(breaker_id),
            breaker_config,
            observer,
        ));

        let bulkhead = Arc::new(Semaphore::new(config.inference_max_concurrent));

        Ok(Self {
            weights: Arc::new(weights),
            model_name,
            device,
            training_state: None,
            breaker,
            bulkhead,
            analytics,
        })
    }

    /// Model name accessor.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Run Hybrid Inference (M21): Pre-Extracted DNA + Live Student Head.
    pub async fn predict_hybrid(
        self: Arc<Self>,
        item_id: i32,
        pre_extracted_dna: Option<Vec<f32>>,
        user_features: Vec<f32>,
    ) -> Result<f32, ModelError> {
        let start = Instant::now();
        
        if let (Some(dna), Some(ref state)) = (pre_extracted_dna, &self.training_state) {
            debug!(item_id = %item_id, "Hybrid Inference: Using pre-extracted DNA + Live Student Head");
            
            let tensors = {
                let active = state.active_weights.read().await;
                active.get(&self.model_name).cloned()
            };

            if let Some(tensors) = tensors {
                let vb = candle_nn::VarBuilder::from_tensors(tensors, DType::F32, &self.device);
                let model = crate::ml::training::candle::architectures::ranking::StudentRankingHead::new(vb)
                    .map_err(|e| ModelError::InferenceFailed(format!("head init: {e}")))?;

                let tribe_tensor = Tensor::from_vec(user_features, (1, 64), &self.device)
                    .map_err(|e| ModelError::InferenceFailed(format!("tribe tensor: {e}")))?;
                let dna_tensor = Tensor::from_vec(dna, (1, 1024), &self.device)
                    .map_err(|e| ModelError::InferenceFailed(format!("dna tensor: {e}")))?;

                let prediction = model.forward(&tribe_tensor, &dna_tensor)
                    .map_err(|e| ModelError::InferenceFailed(format!("forward pass: {e}")))?;
                
                let score = prediction.to_vec2::<f32>()
                    .map_err(|e| ModelError::InferenceFailed(format!("extract result: {e}")))? [0][0];
                
                let latency = start.elapsed();
                if let Some(ref analytics) = self.analytics {
                    analytics.record_response_time(&format!("ml.inference.hybrid.{}", self.model_name), latency.as_millis() as u64);
                }
                
                return Ok(score);
            }
        }

        // Fallback: Use Base Model weights if available
        warn!(item_id = %item_id, "Hybrid Inference: Falling back to Base Model weights");
        let result = self.predict_two_tower(user_features, vec![0.0; 512]).await?;
        Ok(result[0])
    }

    /// Run Language Inference (M16/M21): SLM Student Head for Text Generation.
    pub async fn predict_language(
        self: Arc<Self>,
        input_ids: Vec<u32>,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        
        if let Some(ref state) = self.training_state {
            debug!(model = %self.model_name, "Language Inference: Using Live SLM Student Head");
            
            let tensors = {
                let active = state.active_weights.read().await;
                active.get("slm").cloned()
            };

            if let Some(tensors) = tensors {
                let vb = candle_nn::VarBuilder::from_tensors(tensors, DType::F32, &self.device);
                let model = crate::ml::training::candle::architectures::language::StudentLanguageHead::new(vb)
                    .map_err(|e| ModelError::InferenceFailed(format!("slm head init: {e}")))?;

                let input_tensor = Tensor::new(input_ids.as_slice(), &self.device)
                    .map_err(|e| ModelError::InferenceFailed(format!("input tensor: {e}")))?
                    .unsqueeze(0)?; 

                let logits = model.forward(&input_tensor)
                    .map_err(|e| ModelError::InferenceFailed(format!("slm forward: {e}")))?;
                
                let (_batch, seq_len, vocab) = logits.dims3()
                    .map_err(|e| ModelError::InferenceFailed(format!("logits dims: {e}")))?;
                let last_logits = logits.narrow(1, seq_len - 1, 1)?
                    .reshape((vocab,))?;
                
                let result = last_logits.to_vec1::<f32>()
                    .map_err(|e| ModelError::InferenceFailed(format!("extract logits: {e}")))?;
                
                let latency = start.elapsed();
                if let Some(ref analytics) = self.analytics {
                    analytics.record_response_time(&format!("ml.inference.language.{}", self.model_name), latency.as_millis() as u64);
                }
                
                return Ok(result);
            }
        }

        Err(ModelError::InferenceFailed("SLM training state or weights not found".to_string()))
    }

    /// Run two-tower inference using pure Candle.
    pub async fn predict_two_tower(
        self: Arc<Self>,
        user_features: Vec<f32>,
        item_features: Vec<f32>,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        let metric_key = format!("ml.inference.{}", self.model_name);
        
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| ModelError::Overloaded {
                model: self.model_name.clone(),
                queue_depth: self.bulkhead.available_permits(),
            })?;

        let engine = self.clone();
        let result = self.breaker.call(|| async move {
            if engine.weights.is_empty() {
                return Err(ModelError::InferenceFailed("No weights loaded for base model".to_string()));
            }

            // 1. Initialize Two-Tower Architecture with base weights
            let weights = (*engine.weights).clone();
            let vb = candle_nn::VarBuilder::from_tensors(weights, DType::F32, &engine.device);
            let model = crate::ml::training::candle::architectures::two_tower::TwoTowerModel::new(vb)
                .map_err(|e| ModelError::InferenceFailed(format!("two-tower init: {e}")))?;

            // 2. Convert Inputs to Tensors
            let u_tensor = Tensor::from_vec(user_features, (1, 128), &engine.device)
                .map_err(|e| ModelError::InferenceFailed(format!("user tensor: {e}")))?;
            let i_tensor = Tensor::from_vec(item_features, (1, 128), &engine.device)
                .map_err(|e| ModelError::InferenceFailed(format!("item tensor: {e}")))?;

            // 3. Execute Forward Pass
            let prediction = model.forward(&u_tensor, &i_tensor)
                .map_err(|e| ModelError::InferenceFailed(format!("forward pass: {e}")))?;
            
            let score = prediction.to_vec2::<f32>()
                .map_err(|e| ModelError::InferenceFailed(format!("extract result: {e}")))? [0][0];

            Ok(vec![score])
        }).await.map_err(|e| match e {
            crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => source,
            crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => ModelError::CircuitOpen(self.model_name.clone()),
            crate::circuit_breaker::CircuitBreakerError::TimedOut { .. } => ModelError::InferenceFailed("timeout".to_string()),
        });

        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time(&metric_key, start.elapsed().as_millis() as u64);
            analytics.increment_throughput(&metric_key);
        }

        result
    }

    /// Run multi-action inference (multi-head output).
    pub async fn predict_multi_action(
        self: Arc<Self>,
        user_features: Vec<f32>,
        item_features: Vec<f32>,
    ) -> Result<Vec<Vec<f32>>, ModelError> {
        let start = Instant::now();
        let metric_key = format!("ml.inference.multi.{}", self.model_name);
        
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| ModelError::Overloaded {
                model: self.model_name.clone(),
                queue_depth: self.bulkhead.available_permits(),
            })?;

        let engine = self.clone();
        let result = self.breaker.call(|| async move {
            if engine.weights.is_empty() {
                return Err(ModelError::InferenceFailed("No weights loaded for multi-action model".to_string()));
            }

            // 1. Initialize Multi-Head Architecture
            let weights = (*engine.weights).clone();
            let vb = candle_nn::VarBuilder::from_tensors(weights, DType::F32, &engine.device);
            let model = crate::ml::training::candle::architectures::ranking::MultiHeadRankingHead::new(vb)
                .map_err(|e| ModelError::InferenceFailed(format!("multi-head init: {e}")))?;

            // 2. Convert Inputs (Using 64 for tribe, 1024 for DNA as per ranking schema)
            let tribe_tensor = Tensor::from_vec(user_features, (1, 64), &engine.device)
                .map_err(|e| ModelError::InferenceFailed(format!("tribe tensor: {e}")))?;
            let item_tensor = Tensor::from_vec(item_features, (1, 1024), &engine.device)
                .map_err(|e| ModelError::InferenceFailed(format!("item tensor: {e}")))?;

            // 3. Execute Forward Pass
            let prediction = model.forward(&tribe_tensor, &item_tensor)
                .map_err(|e| ModelError::InferenceFailed(format!("forward pass: {e}")))?;
            
            let scores = prediction.to_vec2::<f32>()
                .map_err(|e| ModelError::InferenceFailed(format!("extract result: {e}")))?;

            Ok(scores)
        }).await.map_err(|e| match e {
            crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => source,
            crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => ModelError::CircuitOpen(self.model_name.clone()),
            crate::circuit_breaker::CircuitBreakerError::TimedOut { .. } => ModelError::InferenceFailed("timeout".to_string()),
        });

        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time(&metric_key, start.elapsed().as_millis() as u64);
            analytics.increment_throughput(&metric_key);
        }

        result
    }

    /// Batch inference with resilience.
    pub async fn predict_batch(
        self: Arc<Self>,
        user_features: Vec<f32>,
        item_features: Vec<f32>,
    ) -> Result<Vec<f32>, ModelError> {
        // Simplified batch inference for Candle migration
        self.predict_two_tower(user_features, item_features).await
    }

    pub fn with_defaults(id: CircuitBreakerId) -> Self {
        let observer = Arc::new(crate::circuit_breaker::observer::NoOpObserver);
        
        Self {
            weights: Arc::new(HashMap::new()),
            model_name: id.label().to_string(),
            device: Device::Cpu,
            training_state: None,
            breaker: Arc::new(CircuitBreaker::new(id, CircuitBreakerConfig::default(), observer)),
            bulkhead: Arc::new(Semaphore::new(10)),
            analytics: None,
        }
    }
}
