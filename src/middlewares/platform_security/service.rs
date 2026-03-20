//! Phase 11: Platform Security Middleware
//!
//! Validates X-Platform and X-Platform-Key headers against configured keys.
//! Logic: If a key is configured for a platform, it is ENFORCED.
//! If no key is configured (empty string), security for that platform is SKIPPED (Open access).
//! Supported Platforms: mobile, web, tv, system, internal

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use crate::config::AppConfig;
use crate::error::AppError;
use tracing::{warn, debug};

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

    // 3. Resolve Expected Key based on Platform
    let config_key = match platform.as_deref() {
        Some("mobile") => Some(&config.security.mobile_api_key),
        Some("web") => Some(&config.security.web_api_key),
        Some("tv") => Some(&config.security.tv_api_key),
        Some("system") => Some(&config.security.system_api_key),
        Some("internal") => Some(&config.security.internal_api_key),
        _ => None,
    };

    // 4. THE CHAMELEON RULE: Provided -> Enforce, Not Provided -> Open
    if let Some(expected) = config_key {
        if expected.is_empty() {
            // No key configured for this platform -> Open access
            debug!(platform = ?platform, "Security: No key configured for platform, skipping enforcement");
            return Ok(next.run(req).await);
        }

        // Key is configured -> Must be provided and match
        if let Some(provided) = platform_key {
            if provided == expected {
                return Ok(next.run(req).await);
            } else {
                warn!(path = %path, platform = ?platform, "Security: Platform key mismatch");
                return Err(AppError::Forbidden("Invalid platform credentials".to_string()));
            }
        } else {
            warn!(path = %path, platform = ?platform, "Security: Missing key for enforced platform");
            return Err(AppError::Unauthorized(format!("X-Platform-Key is required for '{}' platform", platform.unwrap_or_default())));
        }
    }

    // 5. Handle Unknown Platforms
    match (platform.as_deref(), platform_key) {
        (Some(p), _) => {
            warn!(path = %path, platform = %p, "Security: Unauthorized or unknown platform");
            Err(AppError::Forbidden("Unauthorized platform".to_string()))
        }
        _ => {
            // No platform header provided at all
            warn!(path = %path, "Security: Missing platform headers");
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

    // If system_api_key is provided in config, it MUST match.
    // If it's empty, we allow it (for local dev/unsecured admin access) - following user's flexibility rule.
    if config.security.system_api_key.is_empty() {
        return Ok(next.run(req).await);
    }

    if let Some(key) = platform_key {
        if key == config.security.system_api_key {
            return Ok(next.run(req).await);
        } else {
            warn!(path = %path, "Backstage Security: System key mismatch");
            return Err(AppError::Forbidden("Invalid System API key".to_string()));
        }
    }

    warn!(path = %path, "Backstage Security: Missing system credentials");
    Err(AppError::Unauthorized("Missing X-Platform-Key for system access".to_string()))
}
