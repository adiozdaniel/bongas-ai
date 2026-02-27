//! Page layout repository with Netflix-grade resilience patterns.

use std::sync::Arc;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::models::PageLayout;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for managing dynamic page layouts.
pub struct PageLayoutRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl PageLayoutRepository {
    /// Create a new repository.
    pub fn new(
        pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            pool,
            metrics_collector,
        }
    }

    /// Find a page layout by its slug.
    pub async fn find_by_slug(&self, slug: &str) -> AppResult<Option<PageLayout>> {
        let slug = slug.to_string();
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                let row: Option<PageLayout> = sqlx::query_as(
                    "SELECT id, page_slug, scenario_slugs, is_active, created_at, updated_at 
                     FROM page_layouts 
                     WHERE page_slug = $1 AND is_active = true"
                )
                .bind(&slug)
                .fetch_optional(&pool)
                .await?;

                Ok(row)
            })
            .await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create("page_layout");
        metrics.latency.record_duration(duration);
        
        match result {
            Ok(layout) => {
                metrics.successes.increment();
                Ok(layout)
            }
            Err(e) => {
                metrics.failures.increment();
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch page layout by slug: {}", e),
                    source: None,
                }))
            }
        }
    }
}
