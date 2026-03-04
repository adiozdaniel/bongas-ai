//! Activity ingestion backbone — source-agnostic user activity pipeline.
//!
//! Organized into three functional pillars:
//! 📥 RECOVERY (The Stage): Ingestion sources (Kafka, ClickHouse, API).
//! 🎭 PROCESSING (The Backstage): Transformation and enrichment.
//! 🥇 BROADCAST (The Pulse): Downstream delivery and orchestration.

pub mod types;
pub mod recovery;
pub mod processing;
pub mod broadcast;

// Re-export common types for external consumption
pub use types::models::*;
pub use broadcast::manager::service::IngestionManager;
pub use processing::processor::service::ActivityProcessor;
pub use broadcast::metrics::service::{IngestionMetrics, IngestionHealth};
pub use broadcast::producer::service::RecommendationProducer;
