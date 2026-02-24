//! Cross-module configuration validation.

  use crate::config::types::AppConfig;
  use anyhow::{anyhow, Result};

  /// Validate the entire application configuration.
  pub fn validate_app_config(config: &AppConfig) -> Result<()> {
      // Cross-service validation
      if !config.ingestion.kafka.brokers.is_empty() && config.ingestion.kafka.group_id.is_empty() {
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

      // Security validation (Phase 16 Remediation)
      if config.security.mobile_api_key.is_empty() {
          return Err(anyhow!("Security configuration error: mobile_api_key is empty"));
      }
      if config.security.web_api_key.is_empty() {
          return Err(anyhow!("Security configuration error: web_api_key is empty"));
      }
      if config.security.tv_api_key.is_empty() {
          return Err(anyhow!("Security configuration error: tv_api_key is empty"));
      }
      if config.security.system_api_key.is_empty() {
          return Err(anyhow!("Security configuration error: system_api_key is empty"));
      }
      if config.security.jwt_secret_key.is_empty() {
          return Err(anyhow!("Security configuration error: jwt_secret_key is empty"));
      }

      Ok(())
  }
