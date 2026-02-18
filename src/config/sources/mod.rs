
  //! Configuration sources for the Composite Configuration Pattern.
  //!
  //! Provides trait definitions and implementations for loading configuration
  //! from multiple sources with precedence: TOML defaults → ENV overrides → Spring Cloud.
  //!
  //! # Netflix Design Patterns
  //! - **Strategy Pattern**: Each source implements `ConfigSource` trait
  //! - **Composite Pattern**: Sources can be layered with precedence
  //! - **Builder Pattern**: `ConfigLoader` orchestrates source loading
  //! - **Immutable Configuration**: Final config is Arc<AppConfig>

  use std::collections::HashMap;

  /// Configuration source trait for the Strategy Pattern.
  ///
  /// Each implementation provides a different strategy for loading configuration:
  /// - TOML files for defaults
  /// - Environment variables for overrides
  /// - Spring Cloud Config for dynamic configuration
  pub trait ConfigSource: Send + Sync {
      /// Load configuration from this source.
      ///
      /// Returns a HashMap of key-value pairs where keys are dot-separated
      /// paths (e.g., "server.port", "database.url").
      ///
      /// # Returns
      /// - `Ok(HashMap<String, String>)` on success
      /// - `Err(ConfigError)` on failure
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

  /// Configuration source implementations.
  pub mod toml;
  pub mod env;
  pub mod spring_cloud;

  // Re-export commonly used types
  pub use toml::TomlSource;
  pub use env::EnvSource;
  pub use spring_cloud::SpringCloudSource;

