//! Identity extraction middleware.
//! 
//! Responsibilities:
//! 1. Extract IP address and User-Agent.
//! 2. Generate deterministic Device_Hash for anonymous tracking.
//! 3. Manage transparent Visitor_ID via cookies (Zero-Touch).
//! 4. Inject IdentityContext into request extensions.

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

/// Contextual identity information extracted from the request.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IdentityContext {
    pub visitor_id: String,
    pub device_hash: String,
    pub ip_address: String,
    pub user_agent: String,
}

/// Middleware to extract and manage identity context.
pub async fn identity_middleware(mut req: Request<Body>, next: Next) -> Response {
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

    // 2. Extract User-Agent
    let ua = req.headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

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

    let identity = IdentityContext {
        visitor_id: visitor_id.clone(),
        device_hash,
        ip_address: ip,
        user_agent: ua,
    };

    // 5. Inject IdentityContext into extensions
    debug!(visitor_id = %visitor_id, device_hash = %identity.device_hash, "Identity context established");
    req.extensions_mut().insert(identity);

    let mut response = next.run(req).await;

    // 6. Set-Cookie if it's a new visitor (Zero-Touch Persistence)
    if is_new_visitor {
        let cookie_val = format!("visitor_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000", visitor_id);
        if let Ok(value) = header::HeaderValue::from_str(&cookie_val) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }

    response
}
