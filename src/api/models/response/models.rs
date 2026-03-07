//! High-performance, standardized response envelope for all Bongas-AI API endpoints.
//! Follows Netflix-grade architecture patterns for observability, resilience, and consistency.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ─── Response Envelope ──────────────────────────────────────────────────────

/// The unified response envelope for all Bongas-AI API endpoints.
/// Designed for high scalability, fault tolerance, and observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardResponse<T> {
    /// Indicates if the operation was successful.
    pub success: bool,
    /// The actual payload of the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error details if success is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    /// Comprehensive metadata for observability and tracing.
    pub meta: ResponseMeta,
}

/// Rich error details for failed operations, enabling intelligent client-side handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable error message.
    pub message: String,
    /// Domain-specific error code (e.g., "AUTH_001", "DB_CON_ERROR").
    pub code: String,
    /// Error classification (e.g., "VALIDATION", "INTERNAL", "SECURITY").
    pub classification: String,
    /// Whether the client should attempt a retry.
    pub retriable: bool,
    /// Suggested wait time before retry in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    /// Detailed validation or contextual errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, String>>,
}

/// Metadata for tracing, performance monitoring, and version control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Unique identifier for the request, used for log correlation.
    pub request_id: String,
    /// Distributed trace identifier (e.g., Zipkin/Jaeger compatible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// RFC3339 timestamp of when the response was generated.
    pub timestamp: DateTime<Utc>,
    /// Processing duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// API/Service version.
    pub version: String,
}

// ─── Implementation ─────────────────────────────────────────────────────────

impl<T> StandardResponse<T> {
    /// Creates a base response with mandatory metadata.
    fn new(success: bool, data: Option<T>, error: Option<ErrorBody>) -> Self {
        Self {
            success,
            data,
            error,
            meta: ResponseMeta {
                request_id: "pending".to_string(), // Should be set via with_request_id
                trace_id: None,
                timestamp: Utc::now(),
                duration_ms: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Primary constructor for successful responses.
    pub fn success(data: T) -> Self {
        Self::new(true, Some(data), None)
    }

    /// Primary constructor for error responses.
    pub fn error(
        message: impl Into<String>,
        code: impl Into<String>,
        classification: impl Into<String>,
        retriable: bool,
    ) -> Self {
        Self::new(
            false,
            None,
            Some(ErrorBody {
                message: message.into(),
                code: code.into(),
                classification: classification.into(),
                retriable,
                retry_after: None,
                details: None,
            }),
        )
    }

    // ─── Fluent Builders ────────────────────────────────────────────────────

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.meta.request_id = request_id.into();
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.meta.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.meta.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_retry_after(mut self, retry_after_ms: u64) -> Self {
        if let Some(error) = &mut self.error {
            error.retry_after = Some(retry_after_ms);
        }
        self
    }

    pub fn with_error_details(mut self, details: HashMap<String, String>) -> Self {
        if let Some(error) = &mut self.error {
            error.details = Some(details);
        }
        self
    }
}
