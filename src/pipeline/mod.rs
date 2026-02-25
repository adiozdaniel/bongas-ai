//! Recommendation pipeline engine — modular, pluggable, and high-performance.

pub mod executor;
pub mod stages;
pub mod context;
pub mod registry;
pub mod validator;
pub mod optimizer;
pub mod types;

pub use types::*;
pub use executor::PipelineExecutor;
pub use context::ExecutionContext;
pub use registry::PipelineRegistry;
pub use validator::PipelineValidator;
pub use optimizer::PipelineOptimizer;
