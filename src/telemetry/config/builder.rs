use std::collections::HashMap;
use std::path::PathBuf;
use crate::telemetry::config::models::{
    TelemetryConfig, LogLevel, OutputFormat, ExporterType, TelemetryConfigError
};

/// Builder for `TelemetryConfig` with validation.
#[derive(Debug, Clone)]
pub struct TelemetryConfigBuilder {
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

impl TelemetryConfigBuilder {
    pub(super) fn new() -> Self {
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
