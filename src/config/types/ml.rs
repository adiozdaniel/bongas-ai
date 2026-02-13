//! ML configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for machine learning model paths, batch sizes,
//! and ONNX runtime settings.

use std::path::PathBuf;

/// ML configuration.
///
/// Configuration for machine learning model paths, batch sizes,
/// and ONNX runtime settings.
#[derive(Debug, Clone)]
pub struct MlConfig {
    pub model_path: PathBuf,
    pub batch_size: usize,
    pub onnx_enabled: bool,
    pub onnx_execution_provider: String,
    pub onnx_graph_optimization: bool,
    pub feature_store_enabled: bool,
    pub online_learning_enabled: bool,
    pub model_cache_size: usize,
}

impl Default for MlConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("./models"),
            batch_size: 32,
            onnx_enabled: true,
            onnx_execution_provider: "cpu".to_string(),
            onnx_graph_optimization: true,
            feature_store_enabled: true,
            online_learning_enabled: false,
            model_cache_size: 100,
        }
    }
}