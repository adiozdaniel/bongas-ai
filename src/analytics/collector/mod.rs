//! Local statistics collector for client-side metrics.
//!
//! Provides in-memory collection of business statistics without database dependencies.
//! Uses existing resilience patterns for robust operation.

pub mod service;

pub use service::LocalStatsCollector;
