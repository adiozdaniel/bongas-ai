//! Environment variable configuration source for the Composite Configuration Pattern.
//!
//! Loads configuration from environment variables with support for nested structures
//! using underscore-separated keys. Used for configuration overrides.
//!
//! # Netflix Design Patterns
//! - **Strategy Pattern**: Implements `ConfigSource` trait for environment variables
//! - **Composite Pattern**: Supports nested environment variable structures
//! - **Immutable Configuration**: Parses to HashMap for later merging

use super::{ConfigSource, ConfigResult};
use std::collections::HashMap;

/// Environment variable configuration source.
///
/// Loads configuration from environment variables, converting underscore-separated
/// keys to dot-separated paths for easy merging with other sources.
pub struct EnvSource {
    prefix: String,
}

impl EnvSource {
    /// Create a new environment variable source with a prefix.
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
        }
    }

    /// Create a new environment variable source with a specific prefix.
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_uppercase(),
        }
    }

    /// Parse environment variables into a flat HashMap with dot-separated keys.
    fn parse_env_to_flat_map(prefix: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        for (key, value) in std::env::vars() {
            // Skip variables that don't start with our prefix (if prefix is set)
            if !prefix.is_empty() && !key.starts_with(prefix) {
                continue;
            }

            // Remove prefix and convert to lowercase for consistency
            let clean_key = if prefix.is_empty() {
                key.to_lowercase()
            } else {
                key[prefix.len()..].trim_start_matches('_').to_lowercase()
            };

            // Convert underscore-separated to dot-separated
            let dot_key = clean_key.replace('_', ".");
            
            result.insert(dot_key, value);
        }
        
        result
    }
}

impl ConfigSource for EnvSource {
    fn load(&self) -> ConfigResult<HashMap<String, String>> {
        Ok(Self::parse_env_to_flat_map(&self.prefix))
    }
}