//! Configuration source traits and types.

use std::collections::HashMap;

/// Configuration source trait for the Strategy Pattern.
pub trait ConfigSource: Send + Sync {
    /// Load configuration from this source.
    fn load(&self) -> Result<HashMap<String, String>, ConfigError>;

    /// Get the name of the configuration source for logging.
    fn name(&self) -> &'static str;
}

/// Error types for configuration loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("TOML parsing error: {0}")]
    TomlParse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Spring Cloud Config error: {0}")]
    SpringCloud(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration source error: {0}")]
    Source(String),

    #[error("Configuration parse error: {0}")]
    Parse(String),
}

/// Result type for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;
