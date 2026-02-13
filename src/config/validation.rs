//! Cross-module configuration validation.

  use super::types::AppConfig;
  use anyhow::{anyhow, Result};

  /// Validate the entire application configuration.
  pub fn validate_app_config(config: &AppConfig) -> Result<()> {
      // Cross-service validation
      if !config.kafka.brokers.is_empty() && config.kafka.group_id.is_empty() {
          return Err(anyhow!("Kafka configured but missing group_id"));
      }

      // Resource validation (e.g., file paths exist)
      if !config.ml.model_path.exists() {
          return Err(anyhow!("ML model path does not exist: {:?}", config.ml.model_path));
      }

      // Circuit breaker validation
      if config.circuit_breaker.enabled && config.circuit_breaker.failure_rate_threshold == 0.0 {
          return Err(anyhow!("Circuit breaker failure rate threshold must be > 0.0"));
      }

      // Analytics validation
      if config.analytics.enabled && !config.circuit_breaker.enabled {
          return Err(anyhow!("Analytics requires circuit breaker to be enabled"));
      }

      // Error handling validation
      if config.error.enabled && config.error.max_retries == 0 {
          return Err(anyhow!("Error handling requires max_retries > 0"));
      }

      Ok(())
  }
