use tower_http::cors::{CorsLayer, Any};
use axum::http::{Method, HeaderName, HeaderValue};
use std::str::FromStr;

/// Enhanced CORS middleware configuration
pub struct CorsConfig {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<HeaderName>,
    exposed_headers: Vec<HeaderName>,
    allow_credentials: bool,
    max_age: Option<u64>,
}

impl CorsConfig {
    pub fn new() -> Self {
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
}

/// Create default CORS layer for development
pub fn create_dev_cors_layer() -> CorsLayer {
    CorsConfig::new()
        .with_origins(vec![
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
            "http://localhost:8080".to_string(),
            "http://127.0.0.1:8080".to_string(),
        ])
        .allow_credentials(true)
        .build()
}
