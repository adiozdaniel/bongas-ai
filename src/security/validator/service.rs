//! Security configuration validation.
//!
//! Validates `SecurityConfig` fields for consistency and correctness
//! before the SecurityManager is constructed.

use crate::config::SecurityConfig;
use crate::error::SecurityError;

/// Validate the security configuration for consistency.
pub fn validate_security_config(config: &SecurityConfig) -> Result<(), SecurityError> {
    // License key must be present if server validation is enabled
    if config.license_key.is_empty() {
        return Err(SecurityError::LicenseInvalid(
            "Security config: license_key is empty".into(),
        ));
    }

    // Server URL must be present
    if config.license_server_url.is_empty() {
        return Err(SecurityError::ServerValidationFailed {
            reason: "Security config: license_server_url is empty".into(),
        });
    }

    // Circuit breaker failure rates must be 0.0..=1.0
    if config.circuit_breaker_enabled {
        for (name, rate) in [
            ("license_server_failure_rate", config.license_server_failure_rate),
            ("license_server_slow_call_rate", config.license_server_slow_call_rate),
            ("revocation_check_failure_rate", config.revocation_check_failure_rate),
            ("revocation_check_slow_call_rate", config.revocation_check_slow_call_rate),
            ("heartbeat_failure_rate", config.heartbeat_failure_rate),
            ("heartbeat_slow_call_rate", config.heartbeat_slow_call_rate),
        ] {
            if !(0.0..=1.0).contains(&rate) {
                return Err(SecurityError::LicenseInvalid(
                    format!("Security config: {} must be between 0.0 and 1.0, got {}", name, rate),
                ));
            }
        }
    }

    // Bulkhead must allow at least 1 concurrent validation
    if config.max_concurrent_validations == 0 {
        return Err(SecurityError::ValidationOverloaded {
            queue_depth: 0,
        });
    }

    Ok(())
}
