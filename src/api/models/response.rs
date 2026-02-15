//! Standard response envelope and shared request types.

use serde::{Deserialize, Serialize};
use chrono::Utc;

// ─── Request Types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ─── Response Envelope ──────────────────────────────────────────────────────

/// Standard response envelope for all API endpoints.
#[derive(Debug, Serialize)]
pub struct StandardResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
    pub meta: Option<ResponseMeta>,
}

/// Error details for failed responses.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub message: String,
    pub code: String,
    pub classification: String,
    pub retriable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

/// Metadata for successful responses.
#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
}

/// Pagination metadata.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub pages: usize,
}

impl<T> StandardResponse<T> {
    /// Create a successful response.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: Some(ResponseMeta {
                request_id: None,
                timestamp: Utc::now(),
                duration_ms: None,
                pagination: None,
            }),
        }
    }

    /// Create a successful paginated response.
    pub fn paginated(data: T, total: usize, page: usize, per_page: usize) -> Self {
        let pages = if per_page > 0 {
            ((total as f64) / (per_page as f64)).ceil() as usize
        } else {
            0
        };

        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: Some(ResponseMeta {
                request_id: None,
                timestamp: Utc::now(),
                duration_ms: None,
                pagination: Some(PaginationMeta {
                    total,
                    page,
                    per_page,
                    pages,
                }),
            }),
        }
    }

    /// Create an error response.
    pub fn error(
        message: impl Into<String>,
        code: impl Into<String>,
        classification: impl Into<String>,
        retriable: bool,
        retry_after: Option<u64>,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorBody {
                message: message.into(),
                code: code.into(),
                classification: classification.into(),
                retriable,
                retry_after,
            }),
            meta: Some(ResponseMeta {
                request_id: None,
                timestamp: Utc::now(),
                duration_ms: None,
                pagination: None,
            }),
        }
    }

    /// Set request ID in metadata.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        if let Some(meta) = &mut self.meta {
            meta.request_id = Some(request_id.into());
        }
        self
    }

    /// Set duration in metadata.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        if let Some(meta) = &mut self.meta {
            meta.duration_ms = Some(duration_ms);
        }
        self
    }
}
