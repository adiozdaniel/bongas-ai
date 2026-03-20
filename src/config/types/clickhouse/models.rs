//! ClickHouse configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for ClickHouse database connection settings.

/// ClickHouse configuration.
///
/// Configuration for ClickHouse database connection settings including
/// URL, user, password, and database name.
#[derive(Debug, Clone)]
pub struct ClickHouseConfig {
    pub url: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub connection_timeout: u64,
    pub request_timeout: u64,
    pub max_connections: u32,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "".to_string(),
            user: "".to_string(),
            password: "".to_string(),
            database: "".to_string(),
            connection_timeout: 10,
            request_timeout: 60,
            max_connections: 10,
        }
    }
}
