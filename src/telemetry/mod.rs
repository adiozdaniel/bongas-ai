//! Telemetry module for observability infrastructure.
//!
//! Provides centralized configuration and initialization for tracing,
//! logging, and metrics collection across the application.
//!
//! # Features
//! - **Structured Logging**: JSON or pretty format with configurable levels
//! - **Distributed Tracing**: Request ID propagation and span context
//! - **HTTP Middleware**: Automatic request/response tracing for Axum
//! - **Multiple Exporters**: stdout, stderr, file, OTLP (future)
//!
//! # Usage
//!
//! ```rust,ignore
//! use bongas_ai::telemetry::{TelemetryConfig, init};
//!
//! let config = TelemetryConfig::builder()
//!     .service_name("bongas-ai")
//!     .environment("production")
//!     .format(OutputFormat::Json)
//!     .build()?;
//!
//! init(&config)?;
//! ```

pub mod config;
pub mod context;
pub mod exporters;
pub mod middleware;
pub mod tracing;

// ─── Configuration ──────────────────────────────────────────────────────────

pub use config::ExporterType;
pub use config::LogLevel;
pub use config::OutputFormat;
pub use config::TelemetryConfig;
pub use config::TelemetryConfigBuilder;
pub use config::TelemetryConfigError;

// ─── Tracing Setup ──────────────────────────────────────────────────────────

pub use tracing::init;
pub use tracing::init_from_env;
pub use tracing::TracingInitError;

// ─── Context & Correlation ──────────────────────────────────────────────────

pub use context::RequestId;
pub use context::TraceContext;
pub use context::SpanExt;
pub use context::headers;

// ─── Exporters ──────────────────────────────────────────────────────────────

pub use exporters::FileWriter;
pub use exporters::MultiWriter;
pub use exporters::NullWriter;
pub use exporters::BufferedWriter;
pub use exporters::OtlpExporterConfig;

// ─── Middleware ─────────────────────────────────────────────────────────────

pub use middleware::tracing_middleware;
pub use middleware::db_span;
pub use middleware::redis_span;
pub use middleware::kafka_span;
pub use middleware::ml_span;
