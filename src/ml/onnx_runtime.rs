//! Netflix-grade ONNX inference engine with circuit breaker, bulkhead, and analytics.
//!
//! Every inference call is:
//! 1. Guarded by a per-model circuit breaker (trips on failure/slow calls)
//! 2. Limited by a semaphore bulkhead (prevents thread pool exhaustion)
//! 3. Timed out after a configurable duration
//! 4. Recorded in analytics (latency, throughput, errors per model)
//! 5. Classified via `ModelError` → `ErrorClassifier` for resilience routing

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use ndarray::Array2;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::TensorRef;
use tokio::sync::Semaphore;
use tracing::{info, warn, debug};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId};
use crate::circuit_breaker::observer::ResilienceObserver;
use crate::config::MlConfig;
use crate::error::ModelError;

/// ONNX inference engine with Netflix resilience patterns.
///
/// Each engine instance wraps a single ONNX session and is protected by:
/// - **Circuit breaker**: Per-model, trips on inference failures/timeouts
/// - **Bulkhead semaphore**: Limits concurrent inference calls
/// - **Analytics**: Records latency, throughput, and errors
pub struct OnnxInferenceEngine {
    session: Mutex<Session>,
    model_name: String,
    input_names: Vec<String>,
    output_names: Vec<String>,

    // Resilience
    breaker: Arc<CircuitBreaker>,
    bulkhead: Arc<Semaphore>,
    inference_timeout: std::time::Duration,

    // Analytics
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
}

// Session is thread-safe in ONNX Runtime (wraps a thread-safe C++ session).
// By using a Mutex wrapper and &self for inference methods, we satisfy the 
// compiler while allowing safe multi-threaded access.
unsafe impl Sync for OnnxInferenceEngine {}

impl OnnxInferenceEngine {
    /// Create new ONNX inference engine with full resilience wiring.
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
            "Loading ONNX model"
        );

        let session = Session::builder()
            .map_err(|e| ModelError::LoadFailed(format!("session builder: {e}")))?
            .with_optimization_level(if config.onnx_graph_optimization {
                GraphOptimizationLevel::Level3
            } else {
                GraphOptimizationLevel::Disable
            })
            .map_err(|e| ModelError::LoadFailed(format!("optimization level: {e}")))?
            .with_intra_threads(config.onnx_intra_threads)
            .map_err(|e| ModelError::LoadFailed(format!("intra threads: {e}")))?
            .with_memory_pattern(config.onnx_memory_map)
            .map_err(|e| ModelError::LoadFailed(format!("memory pattern: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| ModelError::LoadFailed(format!("commit from file: {e}")))?;

        let input_names: Vec<String> = session.inputs().iter()
            .map(|input| input.name().to_string())
            .collect();

        let output_names: Vec<String> = session.outputs().iter()
            .map(|output| output.name().to_string())
            .collect();

        info!(
            model = %model_name,
            inputs = ?input_names,
            outputs = ?output_names,
            "ONNX model loaded"
        );

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
            session: Mutex::new(session),
            model_name,
            input_names,
            output_names,
            breaker,
            bulkhead,
            inference_timeout: config.inference_timeout,
            analytics,
        })
    }

    /// Model name accessor.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Output names accessor.
    pub fn output_names(&self) -> &[String] {
        &self.output_names
    }

    /// Configured inference timeout for this model.
    pub fn inference_timeout(&self) -> std::time::Duration {
        self.inference_timeout
    }

    /// Circuit breaker health for this model.
    pub fn breaker_health(&self) -> crate::circuit_breaker::CircuitBreakerHealth {
        self.breaker.health()
    }

    /// Run two-tower inference with circuit breaker + bulkhead + analytics.
    pub async fn predict_two_tower(
        &self,
        user_features: Array2<f32>,
        item_features: Array2<f32>,
    ) -> Result<Vec<f32>, ModelError> {
        let start = Instant::now();
        let batch_size = user_features.nrows();
        let metric_key = format!("ml.inference.{}", self.model_name);

        debug!(
            model = %self.model_name,
            batch_size = batch_size,
            "Running ONNX two-tower inference"
        );

        // Bulkhead: try-acquire (non-blocking in sync context)
        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| {
                if let Some(ref analytics) = self.analytics {
                    analytics.increment_error(&metric_key);
                }
                ModelError::Overloaded {
                    model: self.model_name.clone(),
                    queue_depth: self.bulkhead.available_permits(),
                }
            })?;

        // Execute via circuit breaker
        let result = self.breaker.call(|| async {
            self.execute_two_tower(user_features, item_features)
        }).await.map_err(|e| match e {
            crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => source,
            crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => ModelError::CircuitOpen(self.model_name.clone()),
            crate::circuit_breaker::CircuitBreakerError::TimedOut { .. } => ModelError::InferenceFailed("timeout".to_string()),
        });

        let latency = start.elapsed();

        // Analytics
        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time(&metric_key, latency.as_millis() as u64);
            analytics.increment_throughput(&metric_key);
            if result.is_err() {
                analytics.increment_error(&metric_key);
            }
        }

        match &result {
            Ok(scores) => {
                debug!(
                    model = %self.model_name,
                    score_count = scores.len(),
                    latency_ms = latency.as_millis() as u64,
                    "ONNX inference complete"
                );
            }
            Err(e) => {
                warn!(
                    model = %self.model_name,
                    error = %e,
                    latency_ms = latency.as_millis() as u64,
                    "ONNX inference failed"
                );
            }
        }

        result
    }

    /// Batch inference with resilience: circuit breaker + bulkhead + analytics.
    pub async fn predict_batch(
        &self,
        user_features: Vec<Vec<f32>>,
        item_features: Vec<Vec<f32>>,
    ) -> Result<Vec<f32>, ModelError> {
        if user_features.len() != item_features.len() {
            return Err(ModelError::InvalidConfig(format!(
                "user/item feature count mismatch: {} vs {}",
                user_features.len(),
                item_features.len()
            )));
        }

        let batch_size = user_features.len();
        if batch_size == 0 {
            return Ok(Vec::new());
        }

        // Convert Vec<Vec<f32>> to Array2<f32>
        let user_dim = user_features[0].len();
        let item_dim = item_features[0].len();

        let mut user_array = Array2::<f32>::zeros((batch_size, user_dim));
        let mut item_array = Array2::<f32>::zeros((batch_size, item_dim));

        for (i, (user, item)) in user_features.iter().zip(item_features.iter()).enumerate() {
            for (j, &val) in user.iter().enumerate() {
                user_array[[i, j]] = val;
            }
            for (j, &val) in item.iter().enumerate() {
                item_array[[i, j]] = val;
            }
        }

        self.predict_two_tower(user_array, item_array).await
    }

    /// Raw two-tower execution (no resilience wrappers — called inside breaker).
    fn execute_two_tower(
        &self,
        user_features: Array2<f32>,
        item_features: Array2<f32>,
    ) -> Result<Vec<f32>, ModelError> {
        let user_input = TensorRef::from_array_view(user_features.view())
            .map_err(|e| ModelError::InferenceFailed(format!("user tensor: {e}")))?;
        let item_input = TensorRef::from_array_view(item_features.view())
            .map_err(|e| ModelError::InferenceFailed(format!("item tensor: {e}")))?;

        let mut session = self.session.lock().map_err(|_| ModelError::InferenceFailed("session mutex poisoned".to_string()))?;
        
        let outputs = session.run(ort::inputs![
            self.input_names[0].clone() => user_input,
            self.input_names[1].clone() => item_input,
        ])
        .map_err(|e| ModelError::InferenceFailed(format!("session run: {e}")))?;

        // Extract tensor
        let (shape, scores_slice) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| ModelError::InferenceFailed(format!("extract tensor: {e}")))?;

        if shape.is_empty() {
            return Err(ModelError::InferenceFailed("Model returned empty shape".to_string()));
        }

        Ok(scores_slice.to_vec())
    }

    /// Batch inference with resilience: circuit breaker + bulkhead + analytics.
    /// Run multi-action inference (multi-head output).
    /// Returns a Vec of Vecs, where each inner vec contains all predicted action probabilities for an item.
    pub async fn predict_multi_action(
        &self,
        user_features: Array2<f32>,
        item_features: Array2<f32>,
    ) -> Result<Vec<Vec<f32>>, ModelError> {
        let start = Instant::now();
        let batch_size = user_features.nrows();
        let metric_key = format!("ml.inference.multi.{}", self.model_name);

        let _permit = self.bulkhead.clone().try_acquire_owned()
            .map_err(|_| ModelError::Overloaded {
                model: self.model_name.clone(),
                queue_depth: self.bulkhead.available_permits(),
            })?;

        // Execute via circuit breaker
        let result = self.breaker.call(|| async {
            let mut session = self.session.lock().map_err(|_| ModelError::InferenceFailed("session mutex poisoned".to_string()))?;

            let user_input = TensorRef::from_array_view(user_features.view())
                .map_err(|e| ModelError::InferenceFailed(format!("user tensor: {e}")))?;
            let item_input = TensorRef::from_array_view(item_features.view())
                .map_err(|e| ModelError::InferenceFailed(format!("item tensor: {e}")))?;

            let outputs = session.run(ort::inputs![
                self.input_names[0].clone() => user_input,
                self.input_names[1].clone() => item_input,
            ])
            .map_err(|e| ModelError::InferenceFailed(format!("session run: {e}")))?;

            // Extract multi-dimensional output (Batch x Actions)
            let (shape, flat_scores) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| ModelError::InferenceFailed(format!("extract tensor: {e}")))?;

            if shape.len() < 2 {
                return Err(ModelError::InferenceFailed(format!("Unexpected output shape: {:?}", shape)));
            }

            let actions_dim = shape[1] as usize;
            let mut results = Vec::with_capacity(batch_size);
            
            for i in 0..batch_size {
                let start = i * actions_dim;
                let end = start + actions_dim;
                results.push(flat_scores[start..end].to_vec());
            }
            Ok(results)
        }).await.map_err(|e| match e {
            crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => source,
            crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => ModelError::CircuitOpen(self.model_name.clone()),
            crate::circuit_breaker::CircuitBreakerError::TimedOut { .. } => ModelError::InferenceFailed("timeout".to_string()),
        });

        let latency = start.elapsed();
        if let Some(ref analytics) = self.analytics {
            analytics.record_response_time(&metric_key, latency.as_millis() as u64);
            analytics.increment_throughput(&metric_key);
            if result.is_err() {
                analytics.increment_error(&metric_key);
            }
        }

        result
    }
}
