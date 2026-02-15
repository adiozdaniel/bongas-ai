  //! Configuration loader for the Composite Configuration Pattern.
  //!
  //! Orchestrates loading configuration from multiple sources with precedence:
  //! TOML defaults → ENV overrides → Spring Cloud Config. Provides immutable
  //! configuration for Netflix-grade resilience patterns.

use crate::config::types::ingestion;

use super::sources::{ConfigSource, ConfigResult, ConfigError, TomlSource, EnvSource, SpringCloudSource};
use super::types::{
    AppConfig, CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    KafkaConfig, SecurityConfig, MlConfig, PipelineConfig,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

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
          let mut security = SecurityConfig::default();
          
          if let Some(v) = config_map.get("security.license_key") {
              security.license_key = v.clone();
          }
          if let Some(v) = config_map.get("security.license_server_url") {
              security.license_server_url = v.clone();
          }
          if let Some(v) = config_map.get("security.hardware_id_salt") {
              security.hardware_id_salt = v.clone();
          }
          if let Some(v) = config_map.get("security.anti_debug_enabled").and_then(|s| s.parse().ok()) {
              security.anti_debug_enabled = v;
          }
          if let Some(v) = config_map.get("security.binary_protection_enabled").and_then(|s| s.parse().ok()) {
              security.binary_protection_enabled = v;
          }
          if let Some(v) = config_map.get("security.license_validation_interval").and_then(|s| s.parse().ok()) {
              security.license_validation_interval = v;
          }
          
          // Per-layer circuit breaker configuration
          if let Some(v) = config_map.get("security.circuit_breaker_enabled").and_then(|s| s.parse().ok()) {
              security.circuit_breaker_enabled = v;
          }
          if let Some(v) = config_map.get("security.license_server_failure_rate").and_then(|s| s.parse().ok()) {
              security.license_server_failure_rate = v;
          }
          if let Some(v) = config_map.get("security.license_server_slow_call_rate").and_then(|s| s.parse().ok()) {
              security.license_server_slow_call_rate = v;
          }
          if let Some(v) = config_map.get("security.license_server_recovery_timeout").and_then(|s| s.parse::<u64>().ok()) {
              security.license_server_recovery_timeout = Duration::from_secs(v);
          }
          if let Some(v) = config_map.get("security.revocation_check_failure_rate").and_then(|s| s.parse().ok()) {
              security.revocation_check_failure_rate = v;
          }
          if let Some(v) = config_map.get("security.revocation_check_slow_call_rate").and_then(|s| s.parse().ok()) {
              security.revocation_check_slow_call_rate = v;
          }
          if let Some(v) = config_map.get("security.revocation_check_recovery_timeout").and_then(|s| s.parse::<u64>().ok()) {
              security.revocation_check_recovery_timeout = Duration::from_secs(v);
          }
          if let Some(v) = config_map.get("security.heartbeat_failure_rate").and_then(|s| s.parse().ok()) {
              security.heartbeat_failure_rate = v;
          }
          if let Some(v) = config_map.get("security.heartbeat_slow_call_rate").and_then(|s| s.parse().ok()) {
              security.heartbeat_slow_call_rate = v;
          }
          if let Some(v) = config_map.get("security.heartbeat_recovery_timeout").and_then(|s| s.parse::<u64>().ok()) {
              security.heartbeat_recovery_timeout = Duration::from_secs(v);
          }
          
          // Per-layer timeouts
          if let Some(v) = config_map.get("security.server_validation_timeout").and_then(|s| s.parse::<u64>().ok()) {
              security.server_validation_timeout = Duration::from_secs(v);
          }
          if let Some(v) = config_map.get("security.revocation_check_timeout").and_then(|s| s.parse::<u64>().ok()) {
              security.revocation_check_timeout = Duration::from_secs(v);
          }
          if let Some(v) = config_map.get("security.heartbeat_interval").and_then(|s| s.parse::<u64>().ok()) {
              security.heartbeat_interval = Duration::from_secs(v);
          }
          
          // Bulkhead configuration
          if let Some(v) = config_map.get("security.max_concurrent_validations").and_then(|s| s.parse().ok()) {
              security.max_concurrent_validations = v;
          }
          
          // Fallback configuration
          if let Some(v) = config_map.get("security.fallback_on_server_timeout").and_then(|s| s.parse().ok()) {
              security.fallback_on_server_timeout = v;
          }
          if let Some(v) = config_map.get("security.fallback_on_server_error").and_then(|s| s.parse().ok()) {
              security.fallback_on_server_error = v;
          }
          if let Some(v) = config_map.get("security.allow_degraded_mode").and_then(|s| s.parse().ok()) {
              security.allow_degraded_mode = v;
          }
          
          // Analytics configuration
          if let Some(v) = config_map.get("security.analytics_enabled").and_then(|s| s.parse().ok()) {
              security.analytics_enabled = v;
          }
          if let Some(v) = config_map.get("security.analytics_per_layer").and_then(|s| s.parse().ok()) {
              security.analytics_per_layer = v;
          }

          // Parse ML configuration
          let mut ml = MlConfig::default();
          if let Some(v) = config_map.get("ml.model_path") {
              ml.model_path = PathBuf::from(v);
          }
          if let Some(v) = config_map.get("ml.batch_size").and_then(|s| s.parse().ok()) {
              ml.batch_size = v;
          }
          if let Some(v) = config_map.get("onnx.enabled").and_then(|s| s.parse().ok()) {
              ml.onnx_enabled = v;
          }
          if let Some(v) = config_map.get("onnx.execution_provider") {
              ml.onnx_execution_provider = v.clone();
          }
          if let Some(v) = config_map.get("onnx.graph_optimization").and_then(|s| s.parse().ok()) {
              ml.onnx_graph_optimization = v;
          }
          if let Some(v) = config_map.get("onnx.intra_threads").and_then(|s| s.parse().ok()) {
              ml.onnx_intra_threads = v;
          }
          if let Some(v) = config_map.get("ml.feature_store_enabled").and_then(|s| s.parse().ok()) {
              ml.feature_store_enabled = v;
          }
          if let Some(v) = config_map.get("ml.online_learning_enabled").and_then(|s| s.parse().ok()) {
              ml.online_learning_enabled = v;
          }
          if let Some(v) = config_map.get("ml.model_cache_size").and_then(|s| s.parse().ok()) {
              ml.model_cache_size = v;
          }
          if let Some(v) = config_map.get("ml.inference_max_concurrent").and_then(|s| s.parse().ok()) {
              ml.inference_max_concurrent = v;
          }
          if let Some(v) = config_map.get("ml.inference_timeout_ms").and_then(|s| s.parse::<u64>().ok()) {
              ml.inference_timeout = Duration::from_millis(v);
          }
          if let Some(v) = config_map.get("ml.analytics_enabled").and_then(|s| s.parse().ok()) {
              ml.analytics_enabled = v;
          }
          if let Some(v) = config_map.get("ml.canary_enabled").and_then(|s| s.parse().ok()) {
              ml.canary_enabled = v;
          }
          if let Some(v) = config_map.get("ml.canary_traffic_percent").and_then(|s| s.parse().ok()) {
              ml.canary_traffic_percent = v;
          }

          // Parse Pipeline configuration
          let mut pipeline = PipelineConfig::default();
          if let Some(v) = config_map.get("pipeline.stage_breaker_enabled").and_then(|s| s.parse().ok()) {
              pipeline.stage_breaker_enabled = v;
          }
          if let Some(v) = config_map.get("pipeline.stage_timeout_default_ms").and_then(|s| s.parse::<u64>().ok()) {
              pipeline.stage_timeout_default = Duration::from_millis(v);
          }
          if let Some(v) = config_map.get("pipeline.fetch_stage_timeout_ms").and_then(|s| s.parse::<u64>().ok()) {
              pipeline.fetch_stage_timeout = Duration::from_millis(v);
          }
          if let Some(v) = config_map.get("pipeline.ml_stage_timeout_ms").and_then(|s| s.parse::<u64>().ok()) {
              pipeline.ml_stage_timeout = Duration::from_millis(v);
          }
          if let Some(v) = config_map.get("pipeline.pipeline_timeout_ms").and_then(|s| s.parse::<u64>().ok()) {
              pipeline.pipeline_timeout = Duration::from_millis(v);
          }
          if let Some(v) = config_map.get("pipeline.stage_max_concurrent").and_then(|s| s.parse().ok()) {
              pipeline.stage_max_concurrent = v;
          }
          if let Some(v) = config_map.get("pipeline.fallback_enabled").and_then(|s| s.parse().ok()) {
              pipeline.fallback_enabled = v;
          }
          if let Some(v) = config_map.get("pipeline.analytics_enabled").and_then(|s| s.parse().ok()) {
              pipeline.analytics_enabled = v;
          }
          if let Some(v) = config_map.get("pipeline.analytics_per_stage").and_then(|s| s.parse().ok()) {
              pipeline.analytics_per_stage = v;
          }

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
              ingestion,
              security,
              ml,
              pipeline,
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

