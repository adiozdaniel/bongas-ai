//! Statistics uploader with Netflix resilience patterns.
//!
//! Uses existing circuit breaker, bulkhead, and retry patterns from the codebase
//! for robust statistics upload to central server.

pub mod service;
pub mod parquet_exporter;

pub use service::{StatsUploader, UploadError};
pub use parquet_exporter::ParquetExporter;
