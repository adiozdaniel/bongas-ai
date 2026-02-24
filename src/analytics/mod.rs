//! Client-side statistics collection with Netflix resilience patterns.
//!
//! Implements Strategy, Observer, Circuit Breaker, Bulkhead, Decorator, and Composite patterns
//! for robust statistics collection and upload in client binary deployments.

pub mod collector;
pub mod types;
pub mod uploader;

pub use collector::LocalStatsCollector;
pub use types::{ClientStatsPayload, SecurityDetails};
pub use uploader::StatsUploader;
