//! Netflix-grade ML module with resilience patterns.

pub mod onnx_runtime;
pub mod model_loader;
pub mod feature_store;
pub mod embeddings;
pub mod model_registry;
pub mod worker_queue;
pub mod online_learning;
pub mod training_orchestrator;
pub mod utils;

// Re-export key types for public API stability
pub use onnx_runtime::*;
pub use model_loader::*;
pub use feature_store::*;
pub use embeddings::*;
pub use model_registry::*;
pub use worker_queue::*;
pub use online_learning::*;
pub use training_orchestrator::*;
pub use utils::*;
