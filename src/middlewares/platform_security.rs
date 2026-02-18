//! Phase 11: Platform Security Middleware
//!
//! Validates X-Platform and X-Platform-Key headers against configured keys.
//! Supported Platforms: mobile, web, tv, system

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use crate::config::AppConfig;
use tracing::warn;

pub const X_PLATFORM: &str = "X-Platform";
pub const X_PLATFORM_KEY: &str = "X-Platform-Key";

pub async fn platform_security_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Get Config from Extensions
    let config = req.extensions().get::<Arc<AppConfig>>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let path = req.uri().path();
    
    // Skip for health/metrics
    if path.starts_with("/health") || path == "/metrics" {
        return Ok(next.run(req).await);
    }

    // 2. Extract Headers
    let platform = req.headers()
        .get(X_PLATFORM)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase());

    let platform_key = req.headers()
        .get(X_PLATFORM_KEY)
        .and_then(|h| h.to_str().ok());

    match (platform.as_deref(), platform_key) {
        (Some("mobile"), Some(key)) if constant_time_eq(key, &config.security.mobile_api_key) => Ok(next.run(req).await),
        (Some("web"), Some(key)) if constant_time_eq(key, &config.security.web_api_key) => Ok(next.run(req).await),
        (Some("tv"), Some(key)) if constant_time_eq(key, &config.security.tv_api_key) => Ok(next.run(req).await),
        (Some("system"), Some(key)) if constant_time_eq(key, &config.security.system_api_key) => Ok(next.run(req).await),
        (Some(_p), _) => {
            warn!(path = %path, "Invalid platform key or unauthorized platform");
            Err(StatusCode::FORBIDDEN)
        }
        _ => {
            warn!(path = %path, "Missing platform headers");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    
    let mut result = 0;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}
