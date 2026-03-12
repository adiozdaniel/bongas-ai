  //! Cache configuration extracted from AppConfig.

  use std::time::Duration;

  /// Cache configuration.
  #[derive(Debug, Clone)]
  pub struct CacheConfig {
      pub l1_enabled: bool,
      pub l1_max_entries: usize,
      pub l1_ttl: Duration,

      pub l2_enabled: bool,
      pub l2_ttl: Duration,

      pub warming_enabled: bool,
      pub warming_interval: Duration,
      pub warm_scenarios: Vec<String>,
  }

  impl Default for CacheConfig {
      fn default() -> Self {
          Self {
              l1_enabled: true,
              l1_max_entries: 10000,
              l1_ttl: Duration::from_secs(300), // 5 minutes

              l2_enabled: true,
              l2_ttl: Duration::from_secs(3600), // 1 hour retention

              warming_enabled: true,
              warming_interval: Duration::from_secs(1800), // 30 minutes
              warm_scenarios: vec![
                  "personalized_home".to_string(),
                  "continue_watching".to_string(),
                  "trending_now".to_string(),
              ],
          }
      }
  }

  impl CacheConfig {
      /// Create from TOML config values.
      pub fn from_toml_values(
          l1_ttl_seconds: u64,
          l2_ttl_seconds: u64,
          warming_interval_minutes: Option<u64>,
          warm_scenarios: Option<Vec<String>>,
      ) -> Self {
          Self {
              l1_enabled: true,
              l1_max_entries: 10000,
              l1_ttl: Duration::from_secs(l1_ttl_seconds),

              l2_enabled: true,
              l2_ttl: Duration::from_secs(l2_ttl_seconds),

              warming_enabled: warming_interval_minutes.is_some(),
              warming_interval: Duration::from_secs(
                  warming_interval_minutes.unwrap_or(30) * 60
              ),
              warm_scenarios: warm_scenarios.unwrap_or_default(),
          }
      }
  }

