//! Security configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for license validation, hardware binding,
//! and security settings with Netflix-style resilience patterns.

use std::time::Duration;

/// Security configuration with Netflix-style resilience patterns.
///
/// Configuration for license validation, hardware binding, and
/// security settings including license key, server URL, and
/// per-layer circuit breaker, timeout, bulkhead, and fallback settings.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    // Basic security settings
    pub license_key: String,
    pub license_server_url: String,
    pub hardware_id_salt: String,
    pub anti_debug_enabled: bool,
    pub binary_protection_enabled: bool,
    pub license_validation_interval: u64,

    // Per-layer circuit breaker configuration
    pub circuit_breaker_enabled: bool,
    pub license_server_failure_rate: f64,
    pub license_server_slow_call_rate: f64,
    pub license_server_recovery_timeout: Duration,
    pub revocation_check_failure_rate: f64,
    pub revocation_check_slow_call_rate: f64,
    pub revocation_check_recovery_timeout: Duration,
    pub heartbeat_failure_rate: f64,
    pub heartbeat_slow_call_rate: f64,
    pub heartbeat_recovery_timeout: Duration,

    // Per-layer timeouts
    pub server_validation_timeout: Duration,
    pub revocation_check_timeout: Duration,
    pub heartbeat_interval: Duration,

    // Bulkhead configuration
    pub max_concurrent_validations: usize,

    // Fallback configuration
    pub fallback_on_server_timeout: bool,
    pub fallback_on_server_error: bool,
    pub allow_degraded_mode: bool,

    // Analytics configuration
    pub analytics_enabled: bool,
    pub analytics_per_layer: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            // Basic security settings
            license_key: "".to_string(),
            license_server_url: "https://license.example.com".to_string(),
            hardware_id_salt: "".to_string(),
            anti_debug_enabled: true,
            binary_protection_enabled: true,
            license_validation_interval: 3600,

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
