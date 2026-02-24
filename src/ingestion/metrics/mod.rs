//! Ingestion performance metrics and observability.

pub mod service;

pub use service::{IngestionMetrics, IngestionHealth};
pub use crate::ingestion::types::SourceHealth;
