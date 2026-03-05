//! Identity extraction middleware.
//! 
//! Responsibilities:
//! 1. Extract IP address and User-Agent.
//! 2. Generate deterministic Device_Hash for anonymous tracking.
//! 3. Manage transparent Visitor_ID via cookies (Zero-Touch).
//! 4. Inject IdentityContext into request extensions.
//! 5. Reactive Identity Stitching (Anonymous -> Authenticated).

use axum::{
    extract::Request,
    body::Body,
    middleware::Next,
    response::Response,
    http::header,
};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use tracing::debug;
use std::sync::Arc;
use crate::engine::coordination::service::BongasEngine;

/// Contextual identity information extracted from the request.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IdentityContext {
    pub visitor_id: String,
    pub device_hash: String,
    pub device_type: String, // 'mobile', 'tv', 'web', 'tablet', 'all'
    pub ip_address: String,
    pub user_agent: String,
    pub profile_id: Option<String>,
}

/// Middleware to extract and manage identity context.
pub async fn identity_middleware(
    mut req: Request<Body>, 
    next: Next,
    engine: Arc<BongasEngine>,
) -> Response {
    // 1. Extract IP Address
    let ip = req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // 2. Extract User-Agent & Guess Device Type
    let ua = req.headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Device Type Detection logic (Zero-Touch)
    let device_type = req.headers()
        .get("x-device-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| {
            let ua_low = ua.to_lowercase();
            if ua_low.contains("tv") || ua_low.contains("smarttv") || ua_low.contains("googletv") || ua_low.contains("appletv") {
                "tv".to_string()
            } else if ua_low.contains("mobi") || ua_low.contains("iphone") || ua_low.contains("android") && !ua_low.contains("tablet") {
                "mobile".to_string()
            } else if ua_low.contains("tablet") || ua_low.contains("ipad") || ua_low.contains("playbook") {
                "tablet".to_string()
            } else {
                "web".to_string()
            }
        });

    // 3. Generate Device Hash (Deterministic)
    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    hasher.update(ua.as_bytes());
    let device_hash = hex::encode(hasher.finalize());

    // 4. Extract or Generate Visitor ID
    let mut is_new_visitor = false;
    let visitor_id = req.headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find(|cookie| cookie.trim().starts_with("visitor_id="))
                .map(|cookie| cookie.trim()["visitor_id=".len()..].to_string())
        })
        .unwrap_or_else(|| {
            is_new_visitor = true;
            Uuid::new_v4().to_string()
        });

    // 5. Extract Profile ID (Reactive Detection)
    let profile_id = req.headers()
        .get("x-profile-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 6. Reactive Stitching Detection
    if let Some(ref pid) = profile_id {
        // Extract user_id from headers (usually set by an upstream auth gateway or JWT middleware)
        let user_id = req.headers()
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if user_id != 0 {
            // Trigger reactive stitch (Redis-guarded inside service)
            let _ = engine.intelligence.identity.reactive_stitch(
                engine.clone(), 
                &visitor_id, 
                user_id, 
                pid
            ).await;
        }
    }

    let identity = IdentityContext {
        visitor_id: visitor_id.clone(),
        device_hash,
        device_type,
        ip_address: ip,
        user_agent: ua,
        profile_id,
    };

    // 7. Inject IdentityContext into extensions
    debug!(
        visitor_id = %visitor_id, 
        device_hash = %identity.device_hash, 
        profile_id = ?identity.profile_id,
        "Identity context established"
    );
    req.extensions_mut().insert(identity);

    let mut response = next.run(req).await;

    // 8. Set-Cookie if it's a new visitor (Zero-Touch Persistence)
    if is_new_visitor {
        let cookie_val = format!("visitor_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000", visitor_id);
        if let Ok(value) = header::HeaderValue::from_str(&cookie_val) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }

    response
}
