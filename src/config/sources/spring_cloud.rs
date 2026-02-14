//! Spring Cloud Config configuration source for the Composite Configuration Pattern.
//!
//! Loads configuration from Spring Cloud Config server with support for
//! dynamic configuration updates. Used for production configuration management.
//!
//! # Netflix Design Patterns
//! - **Strategy Pattern**: Implements `ConfigSource` trait for Spring Cloud Config
//! - **Composite Pattern**: Supports nested configuration structures
//! - **Immutable Configuration**: Parses to HashMap for later merging

use super::{ConfigSource, ConfigResult, ConfigError};
use std::collections::HashMap;
use reqwest::blocking::Client;
use serde::Deserialize;

/// Spring Cloud Config configuration source.
///
/// Loads configuration from a Spring Cloud Config server, supporting
/// dynamic configuration with application name and profile.
pub struct SpringCloudSource {
    client: Client,
    base_url: String,
    app_name: String,
    profile: String,
    label: Option<String>,
}

impl SpringCloudSource {
    /// Create a new Spring Cloud Config source.
    pub fn new(base_url: String, app_name: String, profile: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            app_name,
            profile,
            label: None,
        }
    }

    /// Set the label (branch/tag) for the configuration.
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Build the configuration URL for Spring Cloud Config.
    fn build_config_url(&self) -> String {
        let mut url = format!("{}/{}/{}", self.base_url, self.app_name, self.profile);

        if let Some(ref label) = self.label {
            url.push('/');
            url.push_str(label);
        }

        url
    }

    /// Parse Spring Cloud Config response into a flat HashMap.
    fn parse_spring_cloud_response(
        response: SpringCloudResponse,
    ) -> ConfigResult<HashMap<String, String>> {
        let mut result = HashMap::new();

        // Process property sources in reverse order (later sources override earlier ones)
        for source in response.property_sources.into_iter().rev() {
            for (key, value) in source.source {
                result.insert(key, value);
            }
        }

        Ok(result)
    }
}

impl ConfigSource for SpringCloudSource {
    fn load(&self) -> ConfigResult<HashMap<String, String>> {
        let url = self.build_config_url();

        tracing::debug!(
            url = %url,
            app = %self.app_name,
            profile = %self.profile,
            "Fetching configuration from Spring Cloud Config"
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .map_err(|e| ConfigError::Source(format!("Spring Cloud Config request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ConfigError::Source(format!(
                "Spring Cloud Config returned status: {}",
                response.status()
            )));
        }

        let spring_response: SpringCloudResponse = response
            .json()
            .map_err(|e| ConfigError::Parse(format!("Failed to parse Spring Cloud Config response: {}", e)))?;

        tracing::info!(
            app = %spring_response.name,
            profiles = ?spring_response.profiles,
            version = ?spring_response.version,
            sources = spring_response.property_sources.len(),
            "Loaded configuration from Spring Cloud Config"
        );

        Self::parse_spring_cloud_response(spring_response)
    }
}

// Helper types for Spring Cloud Config response
#[derive(Debug, Deserialize)]
struct SpringCloudResponse {
    name: String,
    profiles: Vec<String>,
    #[serde(default)]
    #[serde(rename = "label")]
    _label: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    #[serde(rename = "state")]
    _state: Option<String>,
    #[serde(default)]
    property_sources: Vec<PropertySource>,
}

#[derive(Debug, Deserialize)]
struct PropertySource {
    #[serde(default)]
    #[serde(rename = "name")]
    _name: String,
    #[serde(default)]
    source: HashMap<String, String>,
}
