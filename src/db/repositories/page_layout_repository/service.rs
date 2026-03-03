//! Page layout repository with Netflix-grade resilience patterns.

use std::sync::Arc;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::models::PageLayout;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

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
        page_slug: &str, 
        is_landing: bool,
        nav_type: &str,
        composition: serde_json::Value, 
        device_type: Option<String>,
        maturity_rating: Option<String>,
        priority: i32,
        is_active: bool
    ) -> AppResult<PageLayout> {
        let page_slug = page_slug.to_string();
        let nav_type = nav_type.to_string();
        self.pool
            .execute(|pool| async move {
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
                .bind(&page_slug)
                .bind(is_landing)
                .bind(&nav_type)
                .bind(&composition)
                .bind(device_type)
                .bind(maturity_rating)
                .bind(priority)
                .bind(is_active)
                .fetch_one(&pool)
                .await?;

                Ok(row)
            })
            .await
            .map_err(|e| {
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

        self.pool.execute(|pool| async move {
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
        .await
        .map_err(|e| {
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

    /// Find all active page layouts for Nav-Mesh assembly.
    pub async fn find_all_active(&self) -> AppResult<Vec<PageLayout>> {
        self.pool
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
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch active page layouts: {}", e),
                    source: None,
                })
            })
    }
}
