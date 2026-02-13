//! Security configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for license validation, hardware binding,
//! and security settings.

/// Security configuration.
///
/// Configuration for license validation, hardware binding, and
/// security settings including license key and server URL.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub license_key: String,
    pub license_server_url: String,
    pub hardware_id_salt: String,
    pub anti_debug_enabled: bool,
    pub binary_protection_enabled: bool,
    pub license_validation_interval: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            license_key: "".to_string(),
            license_server_url: "https://license.example.com".to_string(),
            hardware_id_salt: "".to_string(),
            anti_debug_enabled: true,
            binary_protection_enabled: true,
            license_validation_interval: 3600,
        }
    }
}