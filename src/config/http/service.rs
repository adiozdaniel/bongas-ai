//! Centralized HTTP configuration for reusability (DRY principle).
//!
//! Used by both middleware and API handlers.

use tower_http::compression::CompressionLayer;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{Method, HeaderName, HeaderValue};
use std::str::FromStr;

// ─── Compression Configuration ─────────────────────────────────────────────

/// Compression configuration for HTTP responses.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum size threshold for compression (default: 1KB)
    pub min_size: u64,
    /// Enable gzip compression
    pub enable_gzip: bool,
    /// Enable brotli compression
    pub enable_brotli: bool,
    /// Enable deflate compression
    pub enable_deflate: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024, // 1KB minimum
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: true,
        }
    }
}

impl CompressionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_size(mut self, size: u64) -> Self {
        self.min_size = size;
        self
    }

    pub fn enable_gzip(mut self, enable: bool) -> Self {
        self.enable_gzip = enable;
        self
    }

    pub fn enable_brotli(mut self, enable: bool) -> Self {
        self.enable_brotli = enable;
        self
    }

    pub fn enable_deflate(mut self, enable: bool) -> Self {
        self.enable_deflate = enable;
        self
    }

    pub fn build(&self) -> CompressionLayer {
        // Use default layer which includes gzip, brotli, and deflate based on accept-encoding
        CompressionLayer::new()
    }
}

// ─── CORS Configuration ────────────────────────────────────────────────────

/// CORS configuration for HTTP endpoints.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<HeaderName>,
    exposed_headers: Vec<HeaderName>,
    allow_credentials: bool,
    max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
                Method::PATCH,
            ],
            allowed_headers: vec![
                HeaderName::from_str("content-type").unwrap(),
                HeaderName::from_str("authorization").unwrap(),
                HeaderName::from_str("x-request-id").unwrap(),
                HeaderName::from_str("x-api-key").unwrap(),
                HeaderName::from_str("accept").unwrap(),
                HeaderName::from_str("origin").unwrap(),
                HeaderName::from_str("user-agent").unwrap(),
            ],
            exposed_headers: vec![
                HeaderName::from_str("x-request-id").unwrap(),
                HeaderName::from_str("x-rate-limit-limit").unwrap(),
                HeaderName::from_str("x-rate-limit-remaining").unwrap(),
                HeaderName::from_str("x-rate-limit-reset").unwrap(),
                HeaderName::from_str("x-cache").unwrap(),
                HeaderName::from_str("x-response-time").unwrap(),
            ],
            allow_credentials: true,
            max_age: Some(3600), // 1 hour
        }
    }
}

impl CorsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    pub fn build(&self) -> CorsLayer {
        let mut cors = CorsLayer::new();

        // Configure allowed origins
        if self.allowed_origins.contains(&"*".to_string()) {
            cors = cors.allow_origin(Any);
        } else {
            let origins: Vec<HeaderValue> = self.allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect();
            cors = cors.allow_origin(origins);
        }

        // Configure allowed methods
        cors = cors.allow_methods(self.allowed_methods.clone());

        // Configure allowed headers
        cors = cors.allow_headers(self.allowed_headers.clone());

        // Configure exposed headers
        cors = cors.expose_headers(self.exposed_headers.clone());

        // Configure credentials
        if self.allow_credentials {
            cors = cors.allow_credentials(true);
        }

        // Configure max age
        if let Some(max_age_seconds) = self.max_age {
            cors = cors.max_age(std::time::Duration::from_secs(max_age_seconds));
        }

        cors
    }

    /// Create CORS layer for development (permissive).
    pub fn dev() -> CorsLayer {
        Self::new()
            .with_origins(vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ])
            .allow_credentials(true)
            .build()
    }
}
