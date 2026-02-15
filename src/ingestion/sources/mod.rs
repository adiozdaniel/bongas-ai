//! Activity sources — pluggable backends for the ingestion layer.

pub mod kafka;
pub mod api;
pub mod clickhouse;

pub use kafka::{KafkaSource, KafkaSourceConfig};
pub use api::ApiSource;
pub use clickhouse::{ClickHouseSource, ClickHouseSourceConfig};
