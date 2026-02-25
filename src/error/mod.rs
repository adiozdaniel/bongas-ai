//! Centralized error module for the Composite Resilience Pattern.

pub mod classification;
pub mod retry;
pub mod context;
pub mod partial;
pub mod domain;
pub mod composite;

// Re-exports for public API stability
pub use classification::*;
pub use retry::*;
pub use context::*;
pub use partial::*;
pub use domain::*;
pub use composite::*;
