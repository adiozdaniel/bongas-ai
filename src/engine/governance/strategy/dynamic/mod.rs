//! Dynamic scenario execution through JSONB-defined pipelines.

pub mod stages;

// Re-export key types for public API stability
pub use stages::*;
