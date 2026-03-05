//! Adaptive Rate Limiting middleware.
//! 
//! Protects the SSE pool by limiting concurrent connections per Visitor_ID.
//! This ensures that a single misbehaving device or bot cannot exhaust
//! the server's connection limits.

use axum::{
    extract::Request,
    body::Body,
    middleware::Next,
    response::{Response, IntoResponse},
    http::StatusCode,
};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::warn;

use crate::api::IdentityContext;
use crate::api::StandardResponse;

/// Tracks active connection counts per visitor.
pub struct ConnectionTracker {
    active_counts: DashMap<String, usize>,
    max_concurrent_per_visitor: usize,
}

impl ConnectionTracker {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            active_counts: DashMap::new(),
            max_concurrent_per_visitor: max_concurrent,
        }
    }

    /// Try to acquire a connection slot for a visitor.
    pub fn acquire(&self, visitor_id: &str) -> bool {
        let mut entry = self.active_counts.entry(visitor_id.to_string()).or_insert(0);
        if *entry < self.max_concurrent_per_visitor {
            *entry += 1;
            true
        } else {
            false
        }
    }

    /// Release a connection slot for a visitor.
    pub fn release(&self, visitor_id: &str) {
        if let Some(mut count) = self.active_counts.get_mut(visitor_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}

/// Middleware to enforce adaptive connection limits.
pub async fn adaptive_limiter_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let tracker = req.extensions().get::<Arc<ConnectionTracker>>().cloned();
    let identity = req.extensions().get::<IdentityContext>().cloned();

    let (tracker, limit_key) = match (tracker, identity) {
        (Some(t), Some(id)) => {
            // Identity-Aware Limiting: Profile ID if authenticated, else Device Hash
            let key = id.profile_id.clone().unwrap_or(id.device_hash.clone());
            (t, key)
        },
        _ => return next.run(req).await, // Skip if tracker or identity is missing
    };

    // 1. Check limit
    if !tracker.acquire(&limit_key) {
        warn!(limit_key = %limit_key, "Resilience Shield: Max concurrent SSE connections reached for this identity");
        let response = StandardResponse::<()>::error(
            "Security Shield: Too many concurrent connections for this profile. Exactly 3 streams allowed.",
            "RATE_LIMIT_EXCEEDED",
            "Security",
            true
        );
        return (StatusCode::TOO_MANY_REQUESTS, axum::Json(response)).into_response();
    }

    // 2. Execute inner layers
    let response = next.run(req).await;

    // 3. Release slot (on response completion)
    tracker.release(&limit_key);

    response
}
