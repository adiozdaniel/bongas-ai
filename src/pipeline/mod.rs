//! Recommendation pipeline engine — modular, pluggable, and high-performance.

pub mod executor;
pub mod recovery;
pub mod processing;
pub mod ranking;
pub mod context;
pub mod registry;
pub mod validator;
pub mod optimizer;
pub mod types;

pub use types::models::*;
pub use types::error::PipelineError;
pub use executor::service::PipelineExecutor;
pub use context::service::ExecutionContext;
pub use registry::service::PipelineRegistry;
pub use validator::service::PipelineValidator;
pub use optimizer::service::PipelineOptimizer;
