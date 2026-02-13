//! Server configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for HTTP server settings including host, port,
//! environment, and security settings.

use std::net::SocketAddr;

/// Server configuration.
///
/// Configuration for HTTP server settings including host, port,
/// environment, and security settings.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub max_connections: usize,
    pub request_timeout: u64,
    pub keep_alive_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            environment: "development".to_string(),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            max_connections: 1000,
            request_timeout: 30,
            keep_alive_timeout: 5,
        }
    }
}

impl ServerConfig {
    /// Get the socket address for the server.
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }
}