use std::collections::HashMap;
use std::path::PathBuf;
use crate::telemetry::config::builder::TelemetryConfigBuilder;

// ─── Validation Error ───────────────────────────────────────────────────────

/// Validation error for telemetry configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryConfigError {
    /// Service name is required.
    MissingServiceName,
    /// Log level is invalid.
    InvalidLogLevel(String),
    /// Sampling rate must be between 0.0 and 1.0.
    InvalidSamplingRate,
    /// File path is invalid for file exporter.
    InvalidFilePath,
    /// OTLP endpoint URL is invalid.
    InvalidOtlpEndpoint,
}

impl std::fmt::Display for TelemetryConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingServiceName => write!(f, "service name is required"),
            Self::InvalidLogLevel(level) => write!(f, "invalid log level: {}", level),
            Self::InvalidSamplingRate => {
                write!(f, "sampling rate must be between 0.0 and 1.0")
            }
            Self::InvalidFilePath => write!(f, "invalid file path for file exporter"),
            Self::InvalidOtlpEndpoint => write!(f, "invalid OTLP endpoint URL"),
        }
    }
}

impl std::error::Error for TelemetryConfigError {}

// ─── Log Level ──────────────────────────────────────────────────────────────

/// Log level for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Off,
}

impl LogLevel {
    /// Convert to tracing level filter string.
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Off => "off",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "off" | "none" => Ok(LogLevel::Off),
            _ => Err(format!("invalid log level: {}", s)),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_filter_str())
    }
}

// ─── Output Format ──────────────────────────────────────────────────────────

/// Output format for logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable pretty format (for development).
    #[default]
    Pretty,
    /// JSON format (for production, log aggregators).
    Json,
    /// Compact single-line format.
    Compact,
}

// ─── Exporter Type ──────────────────────────────────────────────────────────

/// Telemetry export target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExporterType {
    /// Standard output (stdout).
    #[default]
    Stdout,
    /// Standard error (stderr).
    Stderr,
    /// File output.
    File(PathBuf),
    /// OpenTelemetry Protocol (future).
    Otlp { endpoint: String },
    /// No output (discard all).
    None,
}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for the telemetry system.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub(super) service_name: String,
    pub(super) service_version: String,
    pub(super) environment: String,
    pub(super) default_level: LogLevel,
    pub(super) module_levels: HashMap<String, LogLevel>,
    pub(super) format: OutputFormat,
    pub(super) exporter: ExporterType,
    pub(super) sampling_rate: f64,
    pub(super) include_location: bool,
    pub(super) include_target: bool,
    pub(super) include_thread_ids: bool,
    pub(super) include_thread_names: bool,
    pub(super) include_span_events: bool,
    pub(super) static_fields: HashMap<String, String>,
}

impl TelemetryConfig {
    /// Start building a configuration.
    pub fn builder() -> TelemetryConfigBuilder {
        TelemetryConfigBuilder::new()
    }

    #[inline]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    #[inline]
    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    #[inline]
    pub fn environment(&self) -> &str {
        &self.environment
    }

    #[inline]
    pub fn default_level(&self) -> LogLevel {
        self.default_level
    }

    #[inline]
    pub fn module_levels(&self) -> &HashMap<String, LogLevel> {
        &self.module_levels
    }

    #[inline]
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    #[inline]
    pub fn exporter(&self) -> &ExporterType {
        &self.exporter
    }

    #[inline]
    pub fn sampling_rate(&self) -> f64 {
        self.sampling_rate
    }

    #[inline]
    pub fn include_location(&self) -> bool {
        self.include_location
    }

    #[inline]
    pub fn include_target(&self) -> bool {
        self.include_target
    }

    #[inline]
    pub fn include_thread_ids(&self) -> bool {
        self.include_thread_ids
    }

    #[inline]
    pub fn include_thread_names(&self) -> bool {
        self.include_thread_names
    }

    #[inline]
    pub fn include_span_events(&self) -> bool {
        self.include_span_events
    }

    #[inline]
    pub fn static_fields(&self) -> &HashMap<String, String> {
        &self.static_fields
    }

    /// Build the env filter string from configuration.
    pub fn build_filter_string(&self) -> String {
        let mut parts = vec![format!(
            "{}={}",
            self.service_name.replace('-', "_"),
            self.default_level
        )];

        for (module, level) in &self.module_levels {
            parts.push(format!("{}={}", module, level));
        }

        parts.join(",")
    }

    #[inline]
    pub fn is_production(&self) -> bool {
        matches!(
            self.environment.to_lowercase().as_str(),
            "production" | "prod"
        )
    }

    #[inline]
    pub fn is_development(&self) -> bool {
        matches!(
            self.environment.to_lowercase().as_str(),
            "development" | "dev" | "local"
        )
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: String::from("bongas-ai"),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: String::from("development"),
            default_level: LogLevel::Info,
            module_levels: HashMap::new(),
            format: OutputFormat::Pretty,
            exporter: ExporterType::Stdout,
            sampling_rate: 1.0,
            include_location: false,
            include_target: true,
            include_thread_ids: false,
            include_thread_names: false,
            include_span_events: true,
            static_fields: HashMap::new(),
        }
    }
}
