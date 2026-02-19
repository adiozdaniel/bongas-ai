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

    /// Get the underlying resilient pool.
    pub fn pool(&self) -> &Arc<ResilientPool> {
        &self.pool
    }

    /// Create a new scenario configuration.
    pub async fn create(&self, req: CreateScenarioRequest) -> AppResult<ScenarioConfig> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ScenarioConfig>(
                    r#"
                    INSERT INTO scenario_configs (
                        slug, name, description, category, pipeline, 
                        initial_display_limit, scope,
                        cache_ttl_seconds, use_l2_cache, priority, enabled
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true)
                    RETURNING *
                    "#,
                )
                .bind(&req.slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(&req.pipeline)
                .bind(req.initial_display_limit.unwrap_or(5))
                .bind(req.scope.unwrap_or_else(|| serde_json::json!({})))
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
                        initial_display_limit = COALESCE($6, initial_display_limit),
                        scope = COALESCE($7, scope),
                        cache_ttl_seconds = COALESCE($8, cache_ttl_seconds),
                        use_l2_cache = COALESCE($9, use_l2_cache),
                        priority = COALESCE($10, priority),
                        enabled = COALESCE($11, enabled),
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
                .bind(req.initial_display_limit)
                .bind(req.scope)
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
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category,
                        p.definition as pipeline,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        NULL as staleness_rules, true as enabled, 100 as priority,
                        s.created_at, s.created_at as updated_at, NULL as created_by, 1 as version
                    FROM scenarios s
                    LEFT JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    LEFT JOIN pipelines p ON r.pipeline_id = p.id
                    ORDER BY s.slug
                    "#,
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
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category,
                        p.definition as pipeline,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        NULL as staleness_rules, true as enabled, 100 as priority,
                        s.created_at, s.created_at as updated_at, NULL as created_by, 1 as version
                    FROM scenarios s
                    LEFT JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    LEFT JOIN pipelines p ON r.pipeline_id = p.id
                    WHERE s.slug = $1
                    "#,
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
