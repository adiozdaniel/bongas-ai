//! Netflix-grade ML module with resilience patterns.
//!
//! Provides the full ML stack:
//! - **ONNX Runtime**: Inference engine with circuit breaker + bulkhead
//! - **Model Loader**: Retry-with-backoff, fallback to stale model
//! - **Feature Store**: Centralized features with cache + cold-start fallback
//! - **Embeddings**: Embedding manager with cache + zero-vector fallback
//! - **Model Registry**: Versioned models with canary routing + shadow mode
//! - **Worker Queue**: Async task queue with backpressure
//! - **Online Learning**: Batched feedback ingestion with backpressure

pub mod onnx_runtime;
pub mod model_loader;
pub mod feature_store;
pub mod embeddings;
pub mod model_registry;
pub mod worker_queue;
pub mod online_learning;
pub mod training_orchestrator;
pub mod utils;

pub use onnx_runtime::OnnxInferenceEngine;
pub use model_loader::ModelLoader;
pub use feature_store::FeatureStore;
pub use embeddings::EmbeddingManager;
pub use model_registry::{VersionedModelRegistry, ModelVersion, ModelStatus, ModelHealth};
pub use worker_queue::{MlWorkerQueue, MlTask, MlTaskType, MlTaskResult, WorkerQueueStats};
pub use online_learning::{OnlineLearningManager, FeedbackEvent, FeedbackType, OnlineLearningStats};
pub use training_orchestrator::TrainingOrchestrator;
