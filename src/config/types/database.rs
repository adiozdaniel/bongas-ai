//! Database configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for PostgreSQL database connection settings.

/// Database configuration.
///
/// Configuration for PostgreSQL database connection settings including
/// connection URL, pool size, and connection timeouts.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: Option<String>,
    /// Read replica URLs for load distribution
    pub read_replicas: Vec<String>,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub statement_timeout: u64,
    /// Enable read replica load balancing
    pub use_read_replicas: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: None,
            read_replicas: vec![],
            max_connections: 20,
            min_connections: 5,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
            statement_timeout: 300,
            use_read_replicas: false,
        }
    }
}

impl DatabaseConfig {
    /// Get production-grade defaults (10x dev capacity)
    pub fn production() -> Self {
        Self {
            url: None,
            read_replicas: vec![],
            max_connections: 100, // 5x increase for production
            min_connections: 20,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
            statement_timeout: 300,
            use_read_replicas: false,
        }
    }
}