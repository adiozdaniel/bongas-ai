//! Activity ingestion backbone — source-agnostic user activity pipeline.
//!
//! Three sources feed activities through one processor into the staleness engine:
//! - **Kafka** — real-time stream consumer
//! - **API** — direct endpoint for user-reaction calls
//! - **ClickHouse** — polling feedback

pub mod types;
pub mod processor;
pub mod sources;
pub mod metrics;
pub mod producer;
pub mod manager;

pub use manager::IngestionManager;
pub use types::{UserActivity, ActivitySource};
pub use processor::ActivityProcessor;
pub use metrics::{IngestionMetrics, IngestionHealth};
pub use producer::RecommendationProducer;
