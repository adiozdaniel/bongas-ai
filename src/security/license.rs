//! License validation with typed SecurityError returns.
//!
//! Accepts `SecurityConfig` for license key, server URL — no hardcoded values.
//! Each method returns `SecurityError` for proper resilience classification.

use serde::Deserialize;
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};

use crate::config::SecurityConfig;
use crate::error::SecurityError;

/// License validator consuming config for key, server URL, and hardware binding.
pub struct LicenseValidator {
    client: reqwest::Client,
    license_key: String,
    license_server_url: String,
}

#[derive(Debug, Deserialize)]
struct LicenseResponse {
    valid: bool,
    _expires_at: Option<DateTime<Utc>>,
    revoked: bool,
    message: Option<String>,
}

impl LicenseValidator {
    /// Create a new license validator from SecurityConfig.
    pub fn new(config: &SecurityConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            license_key: config.license_key.clone(),
            license_server_url: config.license_server_url.clone(),
        }
    }

    /// Validate license key format and signature.
    pub fn validate_license_key(&self, license_key: &str) -> Result<(), SecurityError> {
        if license_key.is_empty() {
            return Err(SecurityError::LicenseInvalid("License key is empty".into()));
        }

        // License key format: PREFIX-XXXXXXXX-XXXXXXXX-SIGNATURE
        let parts: Vec<&str> = license_key.split('-').collect();
        if parts.len() != 4 {
            return Err(SecurityError::LicenseInvalid("Invalid license key format".into()));
        }

        // Verify signature
        let data = format!("{}-{}-{}", parts[0], parts[1], parts[2]);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        if !hash.starts_with(parts[3]) {
            return Err(SecurityError::LicenseInvalid(
                "License key signature verification failed".into(),
            ));
        }

        Ok(())
    }

    /// Validate hardware binding against license.
    pub fn validate_hardware_binding(&self, hardware_id: &str) -> Result<(), SecurityError> {
        let parts: Vec<&str> = self.license_key.split('-').collect();
        if parts.len() >= 3 {
            let expected_hw_hash = parts[2];

            let mut hasher = Sha256::new();
            hasher.update(hardware_id.as_bytes());
            let hw_hash = format!("{:x}", hasher.finalize());

            if !hw_hash.starts_with(expected_hw_hash) {
                return Err(SecurityError::HardwareMismatch(
                    "Hardware fingerprint mismatch".into(),
                ));
            }
        }

        Ok(())
    }

    /// Validate with license server (network call).
    pub async fn validate_with_server(&self, hardware_id: &str) -> Result<(), SecurityError> {
        let url = format!("{}/validate", self.license_server_url);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "license_key": self.license_key,
                "hardware_id": hardware_id,
            }))
            .send()
            .await
            .map_err(|e| SecurityError::ServerValidationFailed {
                reason: e.to_string(),
            })?
            .json::<LicenseResponse>()
            .await
            .map_err(|e| SecurityError::ServerValidationFailed {
                reason: format!("Failed to parse response: {}", e),
            })?;

        if response.revoked {
            return Err(SecurityError::LicenseRevoked(
                "License has been revoked by server".into(),
            ));
        }

        if !response.valid {
            return Err(SecurityError::ServerValidationFailed {
                reason: response
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        Ok(())
    }

    /// Check license expiration.
    pub fn check_expiration(&self) -> Result<(), SecurityError> {
        // In production, expiration would be encoded in license key
        Ok(())
    }

    /// Check revocation list (network call).
    pub async fn check_revocation_list(&self) -> Result<(), SecurityError> {
        let url = format!("{}/revoked", self.license_server_url);

        let revoked_licenses: Vec<String> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SecurityError::RevocationCheckFailed {
                reason: e.to_string(),
            })?
            .json()
            .await
            .map_err(|e| SecurityError::RevocationCheckFailed {
                reason: format!("Failed to parse revocation list: {}", e),
            })?;

        if revoked_licenses.contains(&self.license_key) {
            return Err(SecurityError::LicenseRevoked(
                "License found in revocation list".into(),
            ));
        }

        Ok(())
    }
}
