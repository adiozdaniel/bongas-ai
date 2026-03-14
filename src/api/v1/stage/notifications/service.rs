use axum::{
    extract::{Query, Extension},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::engine::coordination::service::BongasEngine;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct InboxQuery {
    pub profile_id: String,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct PendingEmailsQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct MarkDispatchedPayload {
    pub email_ids: Vec<i32>,
}

/// GET /notifications/inbox?profile_id=...
pub async fn get_inbox_notifications(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Query(query): Query<InboxQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = query.limit.unwrap_or(20);
    let repo = engine.notifications.repository();
    let notifications = repo.get_inbox_notifications(&query.profile_id, limit).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": notifications
    })))
}

/// GET /emails/pending
pub async fn get_pending_emails(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Query(query): Query<PendingEmailsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = query.limit.unwrap_or(50);
    let repo = engine.notifications.repository();
    let emails = repo.get_pending_emails(limit).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": emails
    })))
}

/// POST /emails/dispatched
pub async fn mark_emails_dispatched(
    Extension(engine): Extension<Arc<BongasEngine>>,
    Json(payload): Json<MarkDispatchedPayload>,
) -> Result<impl IntoResponse, AppError> {
    let repo = engine.notifications.repository();
    if !payload.email_ids.is_empty() {
        repo.mark_emails_dispatched(&payload.email_ids).await?;
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "marked_count": payload.email_ids.len()
    })))
}
