//! Statistics uploader with Netflix resilience patterns.
//!
//! Uses existing circuit breaker, bulkhead, and retry patterns from the codebase
//! for robust statistics upload to central server.

pub mod service;

pub use service::{StatsUploader, UploadError};
