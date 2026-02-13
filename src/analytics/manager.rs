//! The central Analytics Manager.
//! Orchestrates various specialized metric modules into a single registry.

use std::sync::Arc;
use prometheus::{Encoder, Registry, TextEncoder};
use crate::analytics::clickhouse::ClickHouseMetrics;
use crate::analytics::kafka::KafkaMetrics;

pub struct AnalyticsManager {
    registry: Registry,
    pub kafka: KafkaMetrics,
    pub db: ClickHouseMetrics,
}

impl AnalyticsManager {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Pass registry by reference to sub-modules
        let kafka = KafkaMetrics::new(&registry)?;
        let db = ClickHouseMetrics::new(&registry)?;

        Ok(Self {
            registry,
            kafka,
            db,
        })
    }

    /// Gathers all registered metrics into Prometheus text format.
    pub fn gather(&self) -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let _ = encoder.encode(&metric_families, &mut buffer);
        String::from_utf8(buffer).unwrap_or_default()
    }
}

lazy_static::lazy_static! {
    /// Global instance for application-wide telemetry.
    pub static ref ANALYTICS: Arc<AnalyticsManager> = Arc::new(
        AnalyticsManager::new().expect("Failed to initialize analytics registry")
    );
}
