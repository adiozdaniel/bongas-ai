//! Telemetry configuration with Builder pattern and validation.
//!
//! Provides ergonomic, validated configuration for the telemetry system.
//! All fields are private with accessor methods for proper encapsulation.

use std::collections::HashMap;
use std::path::PathBuf;

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
///
/// All fields are private for encapsulation. Use `TelemetryConfig::builder()`
/// for construction and accessor methods for reading values.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    service_name: String,
    service_version: String,
    environment: String,
    default_level: LogLevel,
    module_levels: HashMap<String, LogLevel>,
    format: OutputFormat,
    exporter: ExporterType,
    sampling_rate: f64,
    include_location: bool,
    include_target: bool,
    include_thread_ids: bool,
    include_thread_names: bool,
    include_span_events: bool,
    static_fields: HashMap<String, String>,
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

// ─── Builder ────────────────────────────────────────────────────────────────

/// Builder for `TelemetryConfig` with validation.
#[derive(Debug, Clone)]
pub struct TelemetryConfigBuilder {
    service_name: String,
    service_version: String,
    environment: String,
    default_level: LogLevel,
    module_levels: HashMap<String, LogLevel>,
    format: OutputFormat,
    exporter: ExporterType,
    sampling_rate: f64,
    include_location: bool,
    include_target: bool,
    include_thread_ids: bool,
    include_thread_names: bool,
    include_span_events: bool,
    static_fields: HashMap<String, String>,
}

impl TelemetryConfigBuilder {
    fn new() -> Self {
        let defaults = TelemetryConfig::default();
        Self {
            service_name: defaults.service_name,
            service_version: defaults.service_version,
            environment: defaults.environment,
            default_level: defaults.default_level,
            module_levels: defaults.module_levels,
            format: defaults.format,
            exporter: defaults.exporter,
            sampling_rate: defaults.sampling_rate,
            include_location: defaults.include_location,
            include_target: defaults.include_target,
            include_thread_ids: defaults.include_thread_ids,
            include_thread_names: defaults.include_thread_names,
            include_span_events: defaults.include_span_events,
            static_fields: defaults.static_fields,
        }
    }

    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    pub fn default_level(mut self, level: LogLevel) -> Self {
        self.default_level = level;
        self
    }

    pub fn module_level(mut self, module: impl Into<String>, level: LogLevel) -> Self {
        self.module_levels.insert(module.into(), level);
        self
    }

    pub fn format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    pub fn exporter(mut self, exporter: ExporterType) -> Self {
        self.exporter = exporter;
        self
    }

    pub fn stdout(mut self) -> Self {
        self.exporter = ExporterType::Stdout;
        self
    }

    pub fn stderr(mut self) -> Self {
        self.exporter = ExporterType::Stderr;
        self
    }

    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.exporter = ExporterType::File(path.into());
        self
    }

    pub fn sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate;
        self
    }

    pub fn include_location(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }

    pub fn include_target(mut self, include: bool) -> Self {
        self.include_target = include;
        self
    }

    pub fn include_thread_ids(mut self, include: bool) -> Self {
        self.include_thread_ids = include;
        self
    }

    pub fn include_thread_names(mut self, include: bool) -> Self {
        self.include_thread_names = include;
        self
    }

    pub fn include_span_events(mut self, include: bool) -> Self {
        self.include_span_events = include;
        self
    }

    pub fn static_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.static_fields.insert(key.into(), value.into());
        self
    }

    /// Configure for production environment.
    pub fn production(self) -> Self {
        self.environment("production")
            .format(OutputFormat::Json)
            .default_level(LogLevel::Info)
            .include_location(false)
            .include_thread_ids(false)
    }

    /// Configure for development environment.
    pub fn development(self) -> Self {
        self.environment("development")
            .format(OutputFormat::Pretty)
            .default_level(LogLevel::Debug)
            .include_location(true)
            .include_thread_ids(false)
    }

    pub fn validate(&self) -> Result<(), TelemetryConfigError> {
        if self.service_name.is_empty() {
            return Err(TelemetryConfigError::MissingServiceName);
        }

        if !(0.0..=1.0).contains(&self.sampling_rate) {
            return Err(TelemetryConfigError::InvalidSamplingRate);
        }

        if let ExporterType::File(path) = &self.exporter {
            if path.as_os_str().is_empty() {
                return Err(TelemetryConfigError::InvalidFilePath);
            }
        }

        if let ExporterType::Otlp { endpoint } = &self.exporter {
            if endpoint.is_empty() || !endpoint.starts_with("http") {
                return Err(TelemetryConfigError::InvalidOtlpEndpoint);
            }
        }

        Ok(())
    }

    pub fn build(self) -> Result<TelemetryConfig, TelemetryConfigError> {
        self.validate()?;

        Ok(TelemetryConfig {
            service_name: self.service_name,
            service_version: self.service_version,
            environment: self.environment,
            default_level: self.default_level,
            module_levels: self.module_levels,
            format: self.format,
            exporter: self.exporter,
            sampling_rate: self.sampling_rate,
            include_location: self.include_location,
            include_target: self.include_target,
            include_thread_ids: self.include_thread_ids,
            include_thread_names: self.include_thread_names,
            include_span_events: self.include_span_events,
            static_fields: self.static_fields,
        })
    }
}

impl Default for TelemetryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
