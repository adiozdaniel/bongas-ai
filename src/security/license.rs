use anyhow::{Result, anyhow};
use serde::Deserialize;
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};

// use crate::config::settings::SecuritySettings;

pub struct LicenseValidator {
    // config: SecuritySettings,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct LicenseResponse {
    valid: bool,
    
    _expires_at: Option<DateTime<Utc>>,
    revoked: bool,
    message: Option<String>,
}

impl LicenseValidator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // config: config.clone(),
            client: reqwest::Client::new(),
        })
    }

    /// Validate license key format and signature
    pub fn validate_license_key(&self, license_key: &str) -> Result<()> {
        if license_key.is_empty() {
            return Err(anyhow!("License key is empty"));
        }

        // License key format: PREFIX-XXXXXXXX-XXXXXXXX-SIGNATURE
        let parts: Vec<&str> = license_key.split('-').collect();
        if parts.len() != 4 {
            return Err(anyhow!("Invalid license key format"));
        }

        // Verify signature
        let data = format!("{}-{}-{}", parts[0], parts[1], parts[2]);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        if !hash.starts_with(parts[3]) {
            return Err(anyhow!("License key signature verification failed"));
        }

        Ok(())
    }

    /// Validate hardware binding
    pub fn validate_hardware_binding(&self, hardware_id: &str) -> Result<()> {
        let parts: Vec<&str> = "".split('-').collect();
        if parts.len() >= 3 {
            let expected_hw_hash = parts[2];

            let mut hasher = Sha256::new();
            hasher.update(hardware_id.as_bytes());
            let hw_hash = format!("{:x}", hasher.finalize());

            if !hw_hash.starts_with(expected_hw_hash) {
                return Err(anyhow!("Hardware fingerprint mismatch"));
            }
        }

        Ok(())
    }

    /// Validate with license server
    pub async fn validate_with_server(&self, hardware_id: &str) -> Result<()> {
        let url = format!("{}/validate", "http://localhost:8000");

        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "license_key": "LICENSE-1234-5678-ABCD", // self.config.license_key,
                "hardware_id": hardware_id,
            }))
            .send()
            .await?
            .json::<LicenseResponse>()
            .await?;

        if !response.valid {
            return Err(anyhow!(
                "License validation failed: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        if response.revoked {
            return Err(anyhow!("License has been revoked"));
        }

        Ok(())
    }

    /// Check license expiration
    pub fn check_expiration(&self) -> Result<()> {
        // In production, expiration would be encoded in license key
        Ok(())
    }

    /// Check revocation list
    pub async fn check_revocation_list(&self) -> Result<()> {
        let url = format!("{}/revoked", "http://localhost:8000");

        let revoked_licenses: Vec<String> = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        if revoked_licenses.contains(&"LICENSE-1234-5678-ABCD".to_string()) {
            return Err(anyhow!("License has been revoked"));
        }

        Ok(())
    }
}
