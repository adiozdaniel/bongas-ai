//! Security configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for license validation, hardware binding,
//! and security settings with Netflix-style resilience patterns.

use std::time::Duration;
use serde::Deserialize;

/// Security configuration with Netflix-style resilience patterns.
#[derive(Clone, Deserialize)]
pub struct SecurityConfig {
    // Basic security settings
    #[serde(default)]
    pub license_key: String,
    #[serde(default)]
    pub license_server_url: String,
    #[serde(default)]
    pub hardware_id_salt: String,
    #[serde(default = "default_true")]
    pub anti_debug_enabled: bool,
    #[serde(default = "default_true")]
    pub binary_protection_enabled: bool,
    #[serde(default = "default_interval")]
    pub license_validation_interval: u64,

    // Platform Keys (Phase 11)
    #[serde(default)]
    pub mobile_api_key: String,
    #[serde(default)]
    pub web_api_key: String,
    #[serde(default)]
    pub tv_api_key: String,
    #[serde(default)]
    pub system_api_key: String,
    #[serde(default)]
    pub internal_api_key: String,
    #[serde(default)]
    pub jwt_secret_key: String,

    // Per-layer circuit breaker configuration
    #[serde(default = "default_true")]
    pub circuit_breaker_enabled: bool,
    #[serde(default = "default_failure_rate")]
    pub license_server_failure_rate: f64,
    #[serde(default = "default_failure_rate")]
    pub license_server_slow_call_rate: f64,
    #[serde(default = "default_recovery_timeout")]
    pub license_server_recovery_timeout: Duration,
    #[serde(default = "default_failure_rate")]
    pub revocation_check_failure_rate: f64,
    #[serde(default = "default_failure_rate")]
    pub revocation_check_slow_call_rate: f64,
    #[serde(default = "default_recovery_timeout")]
    pub revocation_check_recovery_timeout: Duration,
    #[serde(default = "default_failure_rate")]
    pub heartbeat_failure_rate: f64,
    #[serde(default = "default_failure_rate")]
    pub heartbeat_slow_call_rate: f64,
    #[serde(default = "default_recovery_timeout")]
    pub heartbeat_recovery_timeout: Duration,

    // Per-layer timeouts
    #[serde(default = "default_validation_timeout")]
    pub server_validation_timeout: Duration,
    #[serde(default = "default_revocation_timeout")]
    pub revocation_check_timeout: Duration,
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: Duration,

    // Bulkhead configuration
    #[serde(default = "default_max_validations")]
    pub max_concurrent_validations: usize,

    // Fallback configuration
    #[serde(default = "default_true")]
    pub fallback_on_server_timeout: bool,
    #[serde(default = "default_true")]
    pub fallback_on_server_error: bool,
    #[serde(default = "default_true")]
    pub allow_degraded_mode: bool,

    // Analytics configuration
    #[serde(default = "default_true")]
    pub analytics_enabled: bool,
    #[serde(default = "default_true")]
    pub analytics_per_layer: bool,
}

fn default_true() -> bool { true }
fn default_interval() -> u64 { 3600 }
fn default_failure_rate() -> f64 { 0.5 }
fn default_recovery_timeout() -> Duration { Duration::from_secs(30) }
fn default_validation_timeout() -> Duration { Duration::from_secs(10) }
fn default_revocation_timeout() -> Duration { Duration::from_secs(5) }
fn default_heartbeat_interval() -> Duration { Duration::from_secs(60) }
fn default_max_validations() -> usize { 10 }

impl std::fmt::Debug for SecurityConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityConfig")
            .field("license_key", &"***REDACTED***")
            .field("license_server_url", &self.license_server_url)
            .field("hardware_id_salt", &"***REDACTED***")
            .field("anti_debug_enabled", &self.anti_debug_enabled)
            .field("binary_protection_enabled", &self.binary_protection_enabled)
            .field("license_validation_interval", &self.license_validation_interval)
            .field("mobile_api_key", &"***REDACTED***")
            .field("web_api_key", &"***REDACTED***")
            .field("tv_api_key", &"***REDACTED***")
            .field("system_api_key", &"***REDACTED***")
            .field("internal_api_key", &"***REDACTED***")
            .field("jwt_secret_key", &"***REDACTED***")
            .field("circuit_breaker_enabled", &self.circuit_breaker_enabled)
            .field("max_concurrent_validations", &self.max_concurrent_validations)
            .field("allow_degraded_mode", &self.allow_degraded_mode)
            .finish()
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            // Basic security settings
            license_key: "".to_string(),
            license_server_url: "".to_string(), // Purged default
            hardware_id_salt: "".to_string(),
            anti_debug_enabled: true,
            binary_protection_enabled: true,
            license_validation_interval: 3600,

            // Platform Keys
            mobile_api_key: "".to_string(),
            web_api_key: "".to_string(),
            tv_api_key: "".to_string(),
            system_api_key: "".to_string(),
            internal_api_key: "".to_string(),
            jwt_secret_key: "".to_string(),

            // Per-layer circuit breaker configuration
            circuit_breaker_enabled: true,
            license_server_failure_rate: 0.5,
            license_server_slow_call_rate: 0.5,
            license_server_recovery_timeout: Duration::from_secs(30),
            revocation_check_failure_rate: 0.5,
            revocation_check_slow_call_rate: 0.5,
            revocation_check_recovery_timeout: Duration::from_secs(30),
            heartbeat_failure_rate: 0.5,
            heartbeat_slow_call_rate: 0.5,
            heartbeat_recovery_timeout: Duration::from_secs(30),

            // Per-layer timeouts
            server_validation_timeout: Duration::from_secs(10),
            revocation_check_timeout: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(60),

            // Bulkhead configuration
            max_concurrent_validations: 10,

            // Fallback configuration
            fallback_on_server_timeout: true,
            fallback_on_server_error: true,
            allow_degraded_mode: true,

            // Analytics configuration
            analytics_enabled: true,
            analytics_per_layer: true,
        }
    }
}
