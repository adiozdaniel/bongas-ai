//! Phase 5: Notification Repository
//!
//! Handles mandatory persistence for notification intents in Postgres and ClickHouse.

use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;
use crate::notification::models::{NotificationIntent, NotificationLedgerEntry};

pub struct NotificationRepository {
    pool: Arc<ResilientPool>,
    clickhouse: Option<clickhouse::Client>,
    metrics: Arc<ResilienceMetricsCollector>,
}

impl NotificationRepository {
    pub fn new(
        pool: Arc<ResilientPool>,
        clickhouse: Option<clickhouse::Client>,
        metrics: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self { pool, clickhouse, metrics }
    }

    /// Persist notification intent to appropriate Postgres table and log to ClickHouse.
    pub async fn persist(&self, intent: &NotificationIntent) -> Result<()> {
        let start = std::time::Instant::now();

        // 1. Postgres Persistence
        match intent {
            NotificationIntent::Email(payload) => {
                self.persist_email(payload).await?;
            }
            NotificationIntent::Inbox(notification) => {
                self.persist_inbox(notification).await?;
            }
        }

        // 2. ClickHouse Ledger (Asynchronous/Fire-and-forget)
        if let Some(ref ch) = self.clickhouse {
            let entry = NotificationLedgerEntry {
                profile_id: intent.profile_id().to_string(),
                notification_type: intent.kind().to_string(),
                template_slug: match intent {
                    NotificationIntent::Email(p) => Some(p.template_slug.clone()),
                    _ => None,
                },
                category: match intent {
                    NotificationIntent::Inbox(n) => Some(n.category.clone()),
                    _ => None,
                },
                status: "pending".to_string(),
                created_at: Utc::now().timestamp(),
            };

            let ch_clone = ch.clone();
            tokio::spawn(async move {
                if let Ok(mut insert) = ch_clone.insert::<NotificationLedgerEntry>("notifications_ledger").await {
                    let _ = insert.write(&entry).await;
                    let _ = insert.end().await;
                }
            });
        }

        // 3. Record Metrics
        let duration = start.elapsed();
        let m = self.metrics.registry().get_or_create("notification.repository.persist");
        m.latency.record_duration(duration);
        m.successes.increment();

        Ok(())
    }

    async fn persist_email(&self, payload: &crate::notification::models::EmailPayload) -> Result<()> {
        let p = payload.clone();
        self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO pending_emails (profile_id, email, subject, body_html, template_slug, metadata, status)
                VALUES ($1, $2, $3, $4, $5, $6, 'pending')
                "#
            )
            .bind(p.profile_id)
            .bind(p.email)
            .bind(p.subject)
            .bind(p.body_html)
            .bind(p.template_slug)
            .bind(p.metadata)
            .execute(&pool)
            .await
        }).await?;
        Ok(())
    }

    async fn persist_inbox(&self, notification: &crate::notification::models::InboxNotification) -> Result<()> {
        let n = notification.clone();
        self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO inbox_notifications (profile_id, title, message, action_url, category, priority, metadata, read)
                VALUES ($1, $2, $3, $4, $5, $6, $7, false)
                "#
            )
            .bind(n.profile_id)
            .bind(n.title)
            .bind(n.message)
            .bind(n.action_url)
            .bind(n.category)
            .bind(n.priority)
            .bind(n.metadata)
            .execute(&pool)
            .await
        }).await?;
        Ok(())
    }
}
