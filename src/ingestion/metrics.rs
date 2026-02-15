//! Per-source ingestion metrics.
//!
//! Provides a unified view of all activity source health, reusing the
//! existing resilience metrics patterns for consistency.

use std::sync::Arc;
use serde::Serialize;

use super::types::{ActivitySource, SourceHealth};

/// Aggregated health across all ingestion sources.
#[derive(Debug, Clone, Serialize)]
pub struct IngestionHealth {
    pub healthy: bool,
    pub total_sources: usize,
    pub active_sources: usize,
    pub degraded_sources: Vec<String>,
    pub total_messages_ingested: u64,
    pub total_errors: u64,
    pub sources: Vec<SourceHealth>,
}

/// Collects metrics from all registered activity sources.
pub struct IngestionMetrics {
    sources: Vec<Arc<dyn ActivitySource>>,
}

impl IngestionMetrics {
    pub fn new(sources: Vec<Arc<dyn ActivitySource>>) -> Self {
        Self { sources }
    }

    /// Get aggregated health across all sources.
    pub async fn health(&self) -> IngestionHealth {
        let mut source_healths = Vec::with_capacity(self.sources.len());
        let mut degraded = Vec::new();
        let mut total_ingested = 0u64;
        let mut total_errors = 0u64;

        for source in &self.sources {
            let h = source.health().await;
            if !h.healthy {
                degraded.push(h.source_name.clone());
            }
            total_ingested += h.messages_ingested;
            total_errors += h.errors;
            source_healths.push(h);
        }

        let active = source_healths.iter().filter(|h| h.healthy).count();

        IngestionHealth {
            healthy: degraded.is_empty(),
            total_sources: self.sources.len(),
            active_sources: active,
            degraded_sources: degraded,
            total_messages_ingested: total_ingested,
            total_errors: total_errors,
            sources: source_healths,
        }
    }
}
