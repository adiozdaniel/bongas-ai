//! Ingestion configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for multi-source activity ingestion including Kafka,
//! API endpoints, and ClickHouse polling.

/// Ingestion configuration.
///
/// Configuration for the activity ingestion backbone with three concurrent sources:
/// - Kafka: Real-time event stream
/// - API: Direct user reaction endpoints
/// - ClickHouse: Polling fallback for analytics data
#[derive(Debug, Clone)]
  pub struct IngestionConfig {
    /// Kafka source configuration
      pub kafka: KafkaSourceConfig,
    /// API source configuration
      pub api: ApiSourceConfig,
    /// ClickHouse polling source configuration
      pub clickhouse: ClickHouseSourceConfig,
    /// Global ingestion settings
      pub buffer_size: usize,
      pub processing_timeout_secs: u64,
  }

/// Kafka source configuration
#[derive(Debug, Clone)]
  pub struct KafkaSourceConfig {
      pub enabled: bool,
      pub brokers: String,
      pub group_id: String,
      pub profile_topic: String,
      pub reaction_topic: String,
      pub notification_topic: String,
      pub playback_topic: String,
      pub connection_timeout: u64,
      pub request_timeout: u64,
      pub max_retries: u32,
      pub retry_backoff: u64,
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
              kafka: KafkaSourceConfig::default(),
              api: ApiSourceConfig::default(),
              clickhouse: ClickHouseSourceConfig::default(),
              buffer_size: 10_000,
              processing_timeout_secs: 30,
          }
      }
  }

  impl Default for KafkaSourceConfig {
      fn default() -> Self {
          Self {
              enabled: true,
              brokers: "localhost:9092".to_string(),
              group_id: "bongas-ai-consumers".to_string(),
              profile_topic: "user.profiles".to_string(),
              reaction_topic: "user.reactions".to_string(),
              notification_topic: "notifications".to_string(),
              playback_topic: "playback.sessions".to_string(),
              connection_timeout: 10,
              request_timeout: 30,
              max_retries: 3,
              retry_backoff: 1000,
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

