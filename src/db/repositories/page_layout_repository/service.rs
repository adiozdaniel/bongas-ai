//! Page layout repository with Netflix-grade resilience patterns.

use std::sync::Arc;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::PageLayout;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Payload for creating or updating a page layout.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageLayoutUpsert {
    pub page_slug: String,
    pub is_landing: bool,
    pub nav_type: String,
    pub composition: serde_json::Value,
    pub device_type: Option<String>,
    pub maturity_rating: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

/// Repository for managing dynamic page layouts with SDUI support.
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

    /// Upsert a page layout.
    pub async fn upsert(
        &self, 
        payload: PageLayoutUpsert,
    ) -> AppResult<PageLayout> {
        let result: AppResult<PageLayout> = self.pool
            .execute(move |pool| async move {
                let row: PageLayout = sqlx::query_as(
                    r#"
                    INSERT INTO page_layouts 
                        (page_slug, is_landing, nav_type, composition, device_type, maturity_rating, priority, is_active, is_deleted, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, NOW())
                    ON CONFLICT (page_slug, device_type, maturity_rating) DO UPDATE
                    SET is_landing = EXCLUDED.is_landing,
                        nav_type = EXCLUDED.nav_type,
                        composition = EXCLUDED.composition,
                        priority = EXCLUDED.priority,
                        is_active = EXCLUDED.is_active,
                        is_deleted = false,
                        updated_at = NOW()
                    RETURNING id, page_slug, is_landing, nav_type, composition, device_type, maturity_rating, priority, is_active, is_deleted, created_at, updated_at
                    "#
                )
                .bind(payload.page_slug)
                .bind(payload.is_landing)
                .bind(payload.nav_type)
                .bind(payload.composition)
                .bind(payload.device_type)
                .bind(payload.maturity_rating)
                .bind(payload.priority)
                .bind(payload.is_active)
                .fetch_one(&pool)
                .await?;

                Ok(row)
            })
            .await;

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to upsert page layout: {}", e),
                source: None,
            })
        })
    }

    /// Soft-delete a page layout.
    pub async fn soft_delete(&self, slug: &str) -> AppResult<bool> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                let result = sqlx::query(
                    "UPDATE page_layouts SET is_active = false, is_deleted = true, updated_at = NOW() WHERE page_slug = $1 AND is_deleted = false"
                )
                .bind(&slug)
                .execute(&pool)
                .await?;

                Ok(result.rows_affected() > 0)
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to soft-delete page layout: {}", e),
                    source: None,
                })
            })
    }

    /// Resolve the singleton landing page for a context.
    pub async fn find_landing_page(
        &self,
        device_type: Option<&str>,
        maturity_rating: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let device = device_type.map(|s| s.to_string());
        let maturity = maturity_rating.map(|s| s.to_string());

        let result: AppResult<Option<PageLayout>> = self.pool.execute(|pool| async move {
            let row: Option<PageLayout> = sqlx::query_as(
                r#"
                SELECT id, page_slug, is_landing, nav_type, composition, device_type, maturity_rating, priority, is_active, is_deleted, created_at, updated_at 
                FROM page_layouts 
                WHERE is_landing = true 
                  AND is_active = true 
                  AND is_deleted = false
                  AND (device_type = $1 OR device_type = 'all')
                  AND (maturity_rating = $2 OR maturity_rating = 'all')
                ORDER BY 
                    (device_type = $1 AND maturity_rating = $2) DESC,
                    (device_type = $1) DESC,
                    (maturity_rating = $2) DESC,
                    priority DESC
                LIMIT 1
                "#
            )
            .bind(device)
            .bind(maturity)
            .fetch_optional(&pool)
            .await?;

            Ok(row)
        })
        .await;

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to resolve landing page: {}", e),
                source: None,
            })
        })
    }

    /// The Targeting Resolver: Find the best layout for a specific slug and context.
    pub async fn find_best_match(
        &self, 
        slug: &str, 
        device_type: Option<&str>, 
        maturity_rating: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let slug = slug.to_string();
        let device = device_type.map(|s| s.to_string());
        let maturity = maturity_rating.map(|s| s.to_string());
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                let row: Option<PageLayout> = sqlx::query_as(
                    r#"
                    SELECT id, page_slug, is_landing, nav_type, composition, device_type, maturity_rating, priority, is_active, is_deleted, created_at, updated_at 
                    FROM page_layouts 
                    WHERE page_slug = $1 
                      AND is_active = true 
                      AND is_deleted = false
                      AND (device_type = $2 OR device_type = 'all')
                      AND (maturity_rating = $3 OR maturity_rating = 'all')
                    ORDER BY 
                        (device_type = $2 AND maturity_rating = $3) DESC,
                        (device_type = $2) DESC,
                        (maturity_rating = $3) DESC,
                        priority DESC
                    LIMIT 1
                    "#
                )
                .bind(&slug)
                .bind(device)
                .bind(maturity)
                .fetch_optional(&pool)
                .await?;

                Ok(row)
            })
            .await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create("page_layout_resolve");
        metrics.latency.record_duration(duration);
        
        match result {
            Ok(layout) => {
                metrics.successes.increment();
                Ok(layout)
            }
            Err(e) => {
                metrics.failures.increment();
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to resolve page layout: {}", e),
                    source: None,
                }))
            }
        }
    }

    pub async fn find_all_active(&self) -> AppResult<Vec<PageLayout>> {
        let result: AppResult<Vec<PageLayout>> = self.pool
            .execute(|pool| async move {
                let rows: Vec<PageLayout> = sqlx::query_as(
                    "SELECT id, page_slug, is_landing, nav_type, composition, device_type, maturity_rating, priority, is_active, is_deleted, created_at, updated_at 
                     FROM page_layouts 
                     WHERE is_active = true AND is_deleted = false
                     ORDER BY nav_type ASC, priority DESC, page_slug ASC"
                )
                .fetch_all(&pool)
                .await?;

                Ok(rows)
            })
            .await;

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to fetch active page layouts: {}", e),
                source: None,
            })
        })
    }
}
