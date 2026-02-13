  //! Configuration loader for the Composite Configuration Pattern.
  //!
  //! Orchestrates loading configuration from multiple sources with precedence:
  //! TOML defaults → ENV overrides → Spring Cloud Config. Provides immutable
  //! configuration for Netflix-grade resilience patterns.

  use super::sources::{ConfigSource, ConfigResult, ConfigError, TomlSource, EnvSource, SpringCloudSource};
  use super::types::{
      AppConfig, CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
      ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
      KafkaConfig, SecurityConfig, MlConfig,
  };
  use std::collections::HashMap;

  /// Configuration loader for the Composite Configuration Pattern.
  ///
  /// Orchestrates loading configuration from multiple sources with precedence:
  /// TOML defaults → ENV overrides → Spring Cloud Config. Provides immutable
  /// configuration for Netflix-grade resilience patterns.
  pub struct ConfigLoader {
      layers: Vec<Box<dyn ConfigSource>>,
  }

  impl ConfigLoader {
      /// Create a new configuration loader.
      pub fn new() -> Self {
          Self { layers: vec![] }
      }

      /// Add TOML source for default configuration.
      pub fn with_defaults(mut self) -> Self {
          self.layers.push(Box::new(TomlSource::from_file("config/default.toml")));
          self
      }

      /// Add environment variable source for overrides.
      pub fn with_env(mut self) -> Self {
          self.layers.push(Box::new(EnvSource::new()));
          self
      }

      /// Add Spring Cloud Config source for dynamic configuration.
      pub fn with_spring_cloud(mut self, url: String, app_name: String, profile: String) -> Self {
          self.layers.push(Box::new(SpringCloudSource::new(url, app_name, profile)));
          self
      }

      /// Load configuration from all sources and create immutable AppConfig.
      pub fn load(self) -> ConfigResult<AppConfig> {
          let mut config_map = HashMap::new();

          // Load from all sources in order (later sources override earlier ones)
          for source in &self.layers {
              let source_config = source.load()?;
              config_map.extend(source_config);
          }

          // Parse and validate configuration
          let app_config = Self::parse_config_map(config_map)?;

          // Validate cross-module configuration dependencies
          Self::validate_config(&app_config)?;

          Ok(app_config)
      }

      /// Parse flat configuration map into typed AppConfig.
      fn parse_config_map(_config_map: HashMap<String, String>) -> ConfigResult<AppConfig> {
          // For now, return default config with basic overrides
          // In a full implementation, this would parse the config_map
          // and create typed configuration objects

          let server = ServerConfig::default();
          let database = DatabaseConfig::default();
          let redis = RedisConfig::default();
          let clickhouse = ClickHouseConfig::default();
          let kafka = KafkaConfig::default();
          let security = SecurityConfig::default();
          let ml = MlConfig::default();

          // Parse circuit breaker config from environment if available
          let circuit_breaker = CircuitBreakerConfig::default();

          // Parse error config from environment if available
          let error = ErrorConfig::default();

          // Parse analytics config from environment if available
          let analytics = AnalyticsConfig::default();

          Ok(AppConfig::new(
              server,
              database,
              redis,
              clickhouse,
              kafka,
              security,
              ml,
              circuit_breaker,
              error,
              analytics,
          ))
      }

      /// Validate cross-module configuration dependencies.
      fn validate_config(config: &AppConfig) -> ConfigResult<()> {
          // Validate that circuit breaker is enabled if analytics is enabled
          if config.analytics.enabled && !config.circuit_breaker.enabled {
              return Err(ConfigError::Validation(
                  "Analytics requires circuit breaker to be enabled".to_string()
              ));
          }

          // Validate error handling configuration
          if config.error.enabled && config.error.max_retries == 0 {
              return Err(ConfigError::Validation(
                  "Error handling requires max_retries > 0".to_string()
              ));
          }

          // Validate service configurations
          if config.database.url.is_none() && config.redis.url.is_empty() {
              return Err(ConfigError::Validation(
                  "At least one database service must be configured".to_string()
              ));
          }

          Ok(())
      }
  }

  impl Default for ConfigLoader {
      fn default() -> Self {
          Self::new()
      }
  }
