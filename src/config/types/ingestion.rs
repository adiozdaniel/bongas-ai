//! Ingestion configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for multi-source activity ingestion including Kafka,
//! API endpoints, and ClickHouse polling.

use super::kafka::KafkaConfig;

/// Ingestion configuration.
///
/// Configuration for the activity ingestion backbone with three concurrent sources:
/// - Kafka: Real-time event stream
/// - API: Direct user reaction endpoints
/// - ClickHouse: Polling fallback for analytics data
#[derive(Debug, Clone)]
  pub struct IngestionConfig {
    /// Kafka source configuration
      pub kafka: KafkaConfig,
    /// API source configuration
      pub api: ApiSourceConfig,
    /// ClickHouse polling source configuration
      pub clickhouse: ClickHouseSourceConfig,
    /// Global ingestion settings
      pub buffer_size: usize,
      pub processing_timeout_secs: u64,
  }

/// API source configuration
#[derive(Debug, Clone)]
  pub struct ApiSourceConfig {
      pub enabled: bool,
      pub rate_limit_per_second: u32,
      pub batch_size: usize,
  }

/// ClickHouse polling source configuration
#[derive(Debug, Clone)]
  pub struct ClickHouseSourceConfig {
      pub enabled: bool,
      pub poll_interval_secs: u64,
      pub batch_size: usize,
      pub lookback_window_secs: u64,
  }

  impl Default for IngestionConfig {
      fn default() -> Self {
          Self {
              kafka: KafkaConfig::default(),
              api: ApiSourceConfig::default(),
              clickhouse: ClickHouseSourceConfig::default(),
              buffer_size: 10_000,
              processing_timeout_secs: 30,
          }
      }
  }

  impl Default for ApiSourceConfig {
      fn default() -> Self {
          Self {
              enabled: true,
              rate_limit_per_second: 1000,
              batch_size: 100,
          }
      }
  }

  impl Default for ClickHouseSourceConfig {
      fn default() -> Self {
          Self {
              enabled: true,
              poll_interval_secs: 60,
              batch_size: 1000,
              lookback_window_secs: 300, // 5 minutes
          }
      }
  }

