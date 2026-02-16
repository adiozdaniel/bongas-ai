//! Scenario repository with Netflix-grade resilience patterns.
//!
//! Provides database access for scenario configurations with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::models::ScenarioConfig;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};
use crate::api::models::scenario::{CreateScenarioRequest, UpdateScenarioRequest};

/// Repository for scenario configuration data with resilience patterns.
pub struct ScenarioRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl ScenarioRepository {
    /// Create a new repository with resilient pool and metrics collector.
    pub fn new(
        pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            pool,
            metrics_collector,
        }
    }

    /// Create a new scenario configuration.
    pub async fn create(&self, req: CreateScenarioRequest) -> AppResult<ScenarioConfig> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    r#"
                    INSERT INTO scenario_configs (
                        slug, name, description, category, pipeline, 
                        cache_ttl_seconds, use_l2_cache, priority, enabled
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)
                    RETURNING *
                    "#,
                )
                .bind(&req.slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(&req.pipeline)
                .bind(req.cache_ttl_seconds)
                .bind(req.use_l2_cache)
                .bind(req.priority)
                .fetch_one(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to create scenario: {}", e),
                    source: None,
                })
            })
    }

    /// Update an existing scenario configuration.
    pub async fn update(&self, slug: &str, req: UpdateScenarioRequest) -> AppResult<ScenarioConfig> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    r#"
                    UPDATE scenario_configs
                    SET name = COALESCE($2, name),
                        description = COALESCE($3, description),
                        category = COALESCE($4, category),
                        pipeline = COALESCE($5, pipeline),
                        cache_ttl_seconds = COALESCE($6, cache_ttl_seconds),
                        use_l2_cache = COALESCE($7, use_l2_cache),
                        priority = COALESCE($8, priority),
                        enabled = COALESCE($9, enabled),
                        updated_at = NOW()
                    WHERE slug = $1
                    RETURNING *
                    "#,
                )
                .bind(&slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(&req.pipeline)
                .bind(req.cache_ttl_seconds)
                .bind(req.use_l2_cache)
                .bind(req.priority)
                .bind(req.enabled)
                .fetch_one(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to update scenario: {}", e),
                    source: None,
                })
            })
    }

    /// Soft-delete a scenario configuration by setting enabled = false.
    pub async fn delete(&self, slug: &str) -> AppResult<()> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                sqlx::query(
                    "UPDATE scenario_configs SET enabled = false, updated_at = NOW() WHERE slug = $1",
                )
                .bind(&slug)
                .execute(&pool)
                .await
            })
            .await
            .map(|_| ())
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to delete scenario: {}", e),
                    source: None,
                })
            })
    }

    /// Load all enabled scenarios from database.
    ///
    /// Executes through circuit breaker with bulkhead protection.
    pub async fn find_all_enabled(&self) -> AppResult<Vec<ScenarioConfig>> {
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    "SELECT * FROM scenario_configs WHERE enabled = true ORDER BY priority DESC",
                )
                .fetch_all(&pool)
                .await
            })
            .await;

        let duration = start_time.elapsed();
        
        match result {
            Ok(scenarios) => {
                // Record successful operation
                let metrics = self.metrics_collector.registry().get_or_create("scenario_config");
                metrics.latency.record_duration(duration);
                metrics.successes.increment();
                Ok(scenarios)
            }
            Err(e) => {
                // Record failed operation
                let metrics = self.metrics_collector.registry().get_or_create("scenario_config");
                metrics.latency.record_duration(duration);
                metrics.failures.increment();
                
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch enabled scenarios: {}", e),
                    source: None,
                }))
            }
        }
    }

    /// Find scenario by slug.
    pub async fn find_by_slug(&self, slug: &str) -> AppResult<Option<ScenarioConfig>> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    "SELECT * FROM scenario_configs WHERE slug = $1",
                )
                .bind(&slug)
                .fetch_optional(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch scenario by slug: {}", e),
                    source: None,
                })
            })
    }

    /// Find scenarios by category.
    pub async fn find_by_category(&self, category: &str) -> AppResult<Vec<ScenarioConfig>> {
        let category = category.to_string();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    "SELECT * FROM scenario_configs WHERE category = $1 AND enabled = true ORDER BY priority DESC",
                )
                .bind(&category)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch scenarios by category: {}", e),
                    source: None,
                })
            })
    }
}
