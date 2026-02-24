//! Hive Mind configuration for global strategy synchronization.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HiveMindConfig {
    /// Whether the Hive Mind connector is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// URL of the central Hive Mind server.
    #[serde(default = "default_hive_mind_url")]
    pub url: String,

    /// API key for authenticating with the Hive Mind.
    pub api_key: Option<String>,

    /// How often to poll for new rules (in seconds).
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,

    /// Whether to automatically approve rules that pass safety checks.
    #[serde(default = "default_auto_approve")]
    pub auto_approve_safe_rules: bool,
}

impl Default for HiveMindConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_hive_mind_url(),
            api_key: None,
            poll_interval_seconds: default_poll_interval(),
            auto_approve_safe_rules: default_auto_approve(),
        }
    }
}

fn default_hive_mind_url() -> String {
    "https://api.bongas.ai/v1/hive-mind".to_string()
}

fn default_poll_interval() -> u64 {
    3600 // 1 hour
}

fn default_auto_approve() -> bool {
    false
}
