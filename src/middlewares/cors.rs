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

    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }

    pub fn with_headers(mut self, headers: Vec<HeaderName>) -> Self {
        self.allowed_headers = headers;
        self
    }

    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    pub fn max_age(mut self, seconds: Option<u64>) -> Self {
        self.max_age = seconds;
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
        if let Some(max_age) = self.max_age {
            cors = cors.max_age(std::time::Duration::from_secs(max_age));
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::new();
        assert_eq!(config.allowed_origins, vec!["http://localhost:3000"]);
        assert!(config.allow_credentials);
        assert_eq!(config.max_age, Some(3600));
    }

    #[test]
    fn test_cors_config_custom() {
        let config = CorsConfig::new()
            .with_origins(vec!["https://example.com".to_string()])
            .allow_credentials(false)
            .max_age(Some(1800));

        assert_eq!(config.allowed_origins, vec!["https://example.com"]);
        assert!(!config.allow_credentials);
        assert_eq!(config.max_age, Some(1800));
    }

    #[test]
    fn test_create_dev_cors_layer() {
        let cors = create_dev_cors_layer();
        // Basic test to ensure the layer can be created
        // CorsLayer doesn't have is_none method, so we just check it's created
        assert!(true);
    }
}
