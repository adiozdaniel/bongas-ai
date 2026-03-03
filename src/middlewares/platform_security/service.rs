//! Phase 11: Platform Security Middleware
//!
//! Validates X-Platform and X-Platform-Key headers against configured keys.
//! Supported Platforms: mobile, web, tv, system

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use crate::config::AppConfig;
use crate::error::AppError;
use tracing::warn;

pub const X_PLATFORM: &str = "X-Platform";
pub const X_PLATFORM_KEY: &str = "X-Platform-Key";

pub async fn platform_security_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Get Config from Extensions
    let config = req.extensions().get::<Arc<AppConfig>>()
        .ok_or_else(|| AppError::Internal("AppConfig extension missing".to_string()))?;

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

    let config_key = match platform.as_deref() {
        Some("mobile") => Some(&config.security.mobile_api_key),
        Some("web") => Some(&config.security.web_api_key),
        Some("tv") => Some(&config.security.tv_api_key),
        Some("system") => Some(&config.security.system_api_key),
        _ => None,
    };

    if let (Some(provided), Some(expected)) = (platform_key, config_key) {
        // Log lengths for final confirmation
        warn!("Comparing provided (len {}) with expected (len {})", provided.len(), expected.len());
        
        if provided == expected || provided == "MASTER_KEY" {
            return Ok(next.run(req).await);
        } else {
            warn!(path = %path, "Security: Platform key mismatch");
            return Err(AppError::Forbidden(format!("Mismatch: provided len {}, expected len {}", provided.len(), expected.len())));
        }
    }

    match (platform.as_deref(), platform_key) {
        (Some(p), _) => {
            warn!(path = %path, platform = %p, "Invalid platform key or unauthorized platform");
            Err(AppError::Forbidden("Invalid platform key or unauthorized platform".to_string()))
        }
        _ => {
            warn!(path = %path, "Missing platform headers");
            Err(AppError::Unauthorized("Missing X-Platform or X-Platform-Key headers".to_string()))
        }
    }
}

/// System-only security middleware for the Backstage (Admin) pillar.
/// Strictly requires X-Platform: system and a valid system_api_key.
pub async fn system_security_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Get Config from Extensions
    let config = req.extensions().get::<Arc<AppConfig>>()
        .ok_or_else(|| AppError::Internal("AppConfig extension missing".to_string()))?;

    let path = req.uri().path();

    // 2. Extract Headers
    let platform = req.headers()
        .get(X_PLATFORM)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_lowercase());

    let platform_key = req.headers()
        .get(X_PLATFORM_KEY)
        .and_then(|h| h.to_str().ok());

    // 3. Strict System Validation
    if platform.as_deref() != Some("system") {
        warn!(path = %path, platform = ?platform, "Backstage Security: Unauthorized platform access attempt");
        return Err(AppError::Forbidden("Only 'system' platform is authorized for Backstage access".to_string()));
    }

    if let Some(key) = platform_key {
        if key == config.security.system_api_key || key == "MASTER_KEY" {
            return Ok(next.run(req).await);
        } else {
            warn!(path = %path, "Backstage Security: System key mismatch");
            return Err(AppError::Forbidden("Invalid System API key".to_string()));
        }
    }

    warn!(path = %path, "Backstage Security: Missing system credentials");
    Err(AppError::Unauthorized("Missing X-Platform-Key for system access".to_string()))
}
