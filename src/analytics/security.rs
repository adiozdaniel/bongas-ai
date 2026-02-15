//! Security validation for client-side statistics.
//!
//! Provides security validation and hardware fingerprinting for client binaries.

use crate::analytics::types::SecurityDetails;

/// Security validator for client-side statistics.
pub struct SecurityValidator {
    /// Client version.
    client_version: String,
    
    /// Hardware fingerprint.
    hardware_fingerprint: String,
}

impl SecurityValidator {
    /// Create a new security validator.
    pub fn new(client_version: String, hardware_fingerprint: String) -> Self {
        Self {
            client_version,
            hardware_fingerprint,
        }
    }

    /// Create with default values.
    pub fn with_defaults() -> Self {
        Self {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            hardware_fingerprint: Self::generate_hardware_fingerprint(),
        }
    }

    /// Get security details for statistics upload.
    pub fn get_security_details(&self, license_valid: bool) -> SecurityDetails {
        SecurityDetails::new(
            license_valid,
            self.hardware_fingerprint.clone(),
            self.client_version.clone(),
        )
    }

    /// Generate hardware fingerprint.
    fn generate_hardware_fingerprint() -> String {
        // In a real implementation, this would collect actual hardware details
        // For now, return a mock fingerprint
        format!("mock-fingerprint-{}", rand::random::<u64>())
    }

    /// Get client version.
    pub fn client_version(&self) -> &str {
        &self.client_version
    }

    /// Get hardware fingerprint.
    pub fn hardware_fingerprint(&self) -> &str {
        &self.hardware_fingerprint
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::with_defaults()
    }
}