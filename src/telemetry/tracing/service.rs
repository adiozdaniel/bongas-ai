//! Tracing setup with layers and subscribers.
//!
//! Provides centralized initialization for the tracing infrastructure.
//! Supports multiple output formats and exporters.

use std::io;
use tracing::Subscriber;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, MakeWriter},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

use crate::telemetry::config::{ExporterType, OutputFormat, TelemetryConfig};

// ─── Initialization ─────────────────────────────────────────────────────────

/// Initialize the global tracing subscriber with the given configuration.
///
/// This should be called once at application startup, before any tracing macros.
///
/// # Errors
/// Returns an error if the subscriber has already been set.
pub fn init(config: &TelemetryConfig) -> Result<(), TracingInitError> {
    build_and_init(config)
}

/// Initialize tracing from environment variables with defaults.
///
/// Uses RUST_LOG for filtering, falls back to "info" level.
pub fn init_from_env() -> Result<(), TracingInitError> {
    let config = TelemetryConfig::default();
    init(&config)
}

/// Build and initialize the subscriber based on configuration.
fn build_and_init(config: &TelemetryConfig) -> Result<(), TracingInitError> {
    let env_filter = build_env_filter(config);

    match config.exporter() {
        ExporterType::Stdout => {
            let layer = build_fmt_layer(config, io::stdout);
            Registry::default()
                .with(env_filter)
                .with(layer)
                .try_init()
                .map_err(|_| TracingInitError::AlreadyInitialized)
        }
        ExporterType::Stderr => {
            let layer = build_fmt_layer(config, io::stderr);
            Registry::default()
                .with(env_filter)
                .with(layer)
                .try_init()
                .map_err(|_| TracingInitError::AlreadyInitialized)
        }
        ExporterType::None => {
            Registry::default()
                .with(env_filter)
                .try_init()
                .map_err(|_| TracingInitError::AlreadyInitialized)
        }
        ExporterType::File(_) | ExporterType::Otlp { .. } => {
            // File and OTLP: fall back to stdout for now
            let layer = build_fmt_layer(config, io::stdout);
            Registry::default()
                .with(env_filter)
                .with(layer)
                .try_init()
                .map_err(|_| TracingInitError::AlreadyInitialized)
        }
    }
}

// ─── Filter Building ────────────────────────────────────────────────────────

/// Build an EnvFilter from configuration.
fn build_env_filter(config: &TelemetryConfig) -> EnvFilter {
    // Try RUST_LOG first, then fall back to config
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.build_filter_string()))
}

// ─── Layer Building ─────────────────────────────────────────────────────────

/// Build a formatting layer with the given writer.
fn build_fmt_layer<S, W>(config: &TelemetryConfig, writer: W) -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let span_events = if config.include_span_events() {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    match config.format() {
        OutputFormat::Json => fmt::layer()
            .json()
            .with_writer(writer)
            .with_target(config.include_target())
            .with_file(config.include_location())
            .with_line_number(config.include_location())
            .with_thread_ids(config.include_thread_ids())
            .with_thread_names(config.include_thread_names())
            .with_span_events(span_events)
            .boxed(),
        OutputFormat::Pretty => fmt::layer()
            .pretty()
            .with_writer(writer)
            .with_target(config.include_target())
            .with_file(config.include_location())
            .with_line_number(config.include_location())
            .with_thread_ids(config.include_thread_ids())
            .with_thread_names(config.include_thread_names())
            .with_span_events(span_events)
            .boxed(),
        OutputFormat::Compact => fmt::layer()
            .compact()
            .with_writer(writer)
            .with_target(config.include_target())
            .with_file(config.include_location())
            .with_line_number(config.include_location())
            .with_thread_ids(config.include_thread_ids())
            .with_thread_names(config.include_thread_names())
            .with_span_events(span_events)
            .boxed(),
    }
}

// ─── Error ──────────────────────────────────────────────────────────────────

/// Error during tracing initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingInitError {
    /// Tracing has already been initialized.
    AlreadyInitialized,
    /// Failed to build subscriber.
    SubscriberBuildFailed(String),
}

impl std::fmt::Display for TracingInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "tracing has already been initialized"),
            Self::SubscriberBuildFailed(msg) => {
                write!(f, "failed to build subscriber: {}", msg)
            }
        }
    }
}

impl std::error::Error for TracingInitError {}
