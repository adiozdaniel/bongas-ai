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
  use std::path::PathBuf;

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
      fn parse_config_map(config_map: HashMap<String, String>) -> ConfigResult<AppConfig> {
          // Parse server configuration
          let server = ServerConfig {
              host: config_map.get("server.host")
                  .cloned()
                  .unwrap_or_else(|| "0.0.0.0".to_string()),
              port: config_map.get("server.port")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(8080),
              environment: config_map.get("server.environment")
                  .cloned()
                  .unwrap_or_else(|| "development".to_string()),
              tls_enabled: config_map.get("server.tls_enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(false),
              tls_cert_path: config_map.get("server.tls_cert_path")
                  .cloned(),
              tls_key_path: config_map.get("server.tls_key_path")
                  .cloned(),
              max_connections: config_map.get("server.max_connections")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(1000),
              request_timeout: config_map.get("server.request_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(30),
              keep_alive_timeout: config_map.get("server.keep_alive_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(5),
          };

          // Parse database configuration
          let database = DatabaseConfig {
              url: config_map.get("database.url")
                  .cloned(),
              max_connections: config_map.get("database.max_connections")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(20),
              min_connections: config_map.get("database.min_connections")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(5),
              connection_timeout: config_map.get("database.connection_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(30),
              idle_timeout: config_map.get("database.idle_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(600),
              max_lifetime: config_map.get("database.max_lifetime")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(1800),
              statement_timeout: config_map.get("database.statement_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(300),
          };

          // Parse Redis configuration
          let redis = RedisConfig {
              url: config_map.get("redis.url")
                  .cloned()
                  .unwrap_or_else(|| "redis://localhost:6379".to_string()),
              pool_size: config_map.get("redis.pool_size")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(10),
              connection_timeout: config_map.get("redis.connection_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(5),
              request_timeout: config_map.get("redis.request_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(10),
              max_retries: config_map.get("redis.max_retries")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(3),
              retry_backoff: config_map.get("redis.retry_backoff")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(100),
          };

          // Parse ClickHouse configuration
          let clickhouse = ClickHouseConfig {
              url: config_map.get("clickhouse.url")
                  .cloned()
                  .unwrap_or_else(|| "http://localhost:8123".to_string()),
              user: config_map.get("clickhouse.user")
                  .cloned()
                  .unwrap_or_else(|| "default".to_string()),
              password: config_map.get("clickhouse.password")
                  .cloned()
                  .unwrap_or_else(|| "".to_string()),
              database: config_map.get("clickhouse.database")
                  .cloned()
                  .unwrap_or_else(|| "baze_analytics".to_string()),
              connection_timeout: config_map.get("clickhouse.connection_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(10),
              request_timeout: config_map.get("clickhouse.request_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(60),
              max_connections: config_map.get("clickhouse.max_connections")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(10),
          };

          // Parse Kafka configuration
          let kafka = KafkaConfig {
              brokers: config_map.get("kafka.brokers")
                  .cloned()
                  .unwrap_or_else(|| "localhost:9092".to_string()),
              group_id: config_map.get("kafka.group_id")
                  .cloned()
                  .unwrap_or_else(|| "bongas-ai-consumers".to_string()),
              profile_topic: config_map.get("kafka.profile_topic")
                  .cloned()
                  .unwrap_or_else(|| "profile.events".to_string()),
              reaction_topic: config_map.get("kafka.reaction_topic")
                  .cloned()
                  .unwrap_or_else(|| "reaction.events".to_string()),
              notification_topic: config_map.get("kafka.notification_topic")
                  .cloned()
                  .unwrap_or_else(|| "notification.events".to_string()),
              playback_topic: config_map.get("kafka.playback_topic")
                  .cloned()
                  .unwrap_or_else(|| "playback.events".to_string()),
              connection_timeout: config_map.get("kafka.connection_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(10),
              request_timeout: config_map.get("kafka.request_timeout")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(30),
              max_retries: config_map.get("kafka.max_retries")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(3),
              retry_backoff: config_map.get("kafka.retry_backoff")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(1000),
          };

          // Parse Security configuration
          let security = SecurityConfig {
              license_key: config_map.get("security.license_key")
                  .cloned()
                  .unwrap_or_else(|| "".to_string()),
              license_server_url: config_map.get("security.license_server_url")
                  .cloned()
                  .unwrap_or_else(|| "https://license.example.com".to_string()),
              hardware_id_salt: config_map.get("security.hardware_id_salt")
                  .cloned()
                  .unwrap_or_else(|| "".to_string()),
              anti_debug_enabled: config_map.get("security.anti_debug_enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(true),
              binary_protection_enabled: config_map.get("security.binary_protection_enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(true),
              license_validation_interval: config_map.get("security.license_validation_interval")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(3600),
          };

          // Parse ML configuration
          let ml = MlConfig {
              model_path: config_map.get("ml.model_path")
                  .map(PathBuf::from)
                  .unwrap_or_else(|| PathBuf::from("./models")),
              batch_size: config_map.get("ml.batch_size")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(32),
              onnx_enabled: config_map.get("onnx.enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(true),
              onnx_execution_provider: config_map.get("onnx.execution_provider")
                  .cloned()
                  .unwrap_or_else(|| "cpu".to_string()),
              onnx_graph_optimization: config_map.get("onnx.graph_optimization")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(true),
              feature_store_enabled: config_map.get("ml.feature_store_enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(true),
              online_learning_enabled: config_map.get("ml.online_learning_enabled")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(false),
              model_cache_size: config_map.get("ml.model_cache_size")
                  .and_then(|s| s.parse().ok())
                  .unwrap_or(100),
          };

          // Circuit breaker, error, and analytics configs use defaults
          // These are configured in code, not in TOML
          let circuit_breaker = CircuitBreakerConfig::default();
          let error = ErrorConfig::default();
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

