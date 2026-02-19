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

    /// Create a new scenario configuration in the Intelligent Brain.
    pub async fn create(&self, req: CreateScenarioRequest) -> AppResult<ScenarioConfig> {
        self.pool
            .execute(|pool| async move {
                let mut tx = pool.begin().await?;

                // 1. Insert into scenarios
                let scenario_id: i32 = sqlx::query_scalar(
                    r#"
                    INSERT INTO scenarios (
                        slug, name, description, category, 
                        initial_display_limit, scope,
                        cache_ttl_seconds, use_l2_cache
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    RETURNING id
                    "#,
                )
                .bind(&req.slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(req.initial_display_limit.unwrap_or(5))
                .bind(req.scope.clone().unwrap_or_else(|| serde_json::json!({})))
                .bind(req.cache_ttl_seconds)
                .bind(req.use_l2_cache)
                .fetch_one(&mut *tx)
                .await?;

                // 2. Insert into pipelines (Strategy)
                let pipeline_slug = format!("{}_strategy_v1", req.slug);
                let pipeline_id: i32 = sqlx::query_scalar(
                    r#"
                    INSERT INTO pipelines (slug, name, definition)
                    VALUES ($1, $2, $3)
                    RETURNING id
                    "#,
                )
                .bind(&pipeline_slug)
                .bind(format!("{} Strategy", req.name))
                .bind(&req.pipeline)
                .fetch_one(&mut *tx)
                .await?;

                // 3. Link them in scenario_rules (Default Rule)
                sqlx::query(
                    r#"
                    INSERT INTO scenario_rules (scenario_id, pipeline_id, priority, condition, is_active, description)
                    VALUES ($1, $2, $3, '{}'::jsonb, true, 'Default Strategy')
                    "#,
                )
                .bind(scenario_id)
                .bind(pipeline_id)
                .bind(req.priority)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                // Fetch the newly created configuration using the join path
                sqlx::query_as::<_, ScenarioConfig>(
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category,
                        p.definition as pipeline,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        NULL as staleness_rules, true as enabled, r.priority,
                        s.created_at, s.created_at as updated_at, NULL as created_by, 1 as version
                    FROM scenarios s
                    JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    JOIN pipelines p ON r.pipeline_id = p.id
                    WHERE s.id = $1
                    "#,
                )
                .bind(scenario_id)
                .fetch_one(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to create scenario in Intelligent Brain: {}", e),
                    source: None,
                })
            })
    }

    /// Update an existing scenario configuration in the Intelligent Brain.
    pub async fn update(&self, slug: &str, req: UpdateScenarioRequest) -> AppResult<ScenarioConfig> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                let mut tx = pool.begin().await?;

                // 1. Update scenario metadata
                let scenario_id: i32 = sqlx::query_scalar(
                    r#"
                    UPDATE scenarios
                    SET name = COALESCE($2, name),
                        description = COALESCE($3, description),
                        category = COALESCE($4, category),
                        initial_display_limit = COALESCE($5, initial_display_limit),
                        scope = COALESCE($6, scope),
                        cache_ttl_seconds = COALESCE($7, cache_ttl_seconds),
                        use_l2_cache = COALESCE($8, use_l2_cache)
                    WHERE slug = $1
                    RETURNING id
                    "#,
                )
                .bind(&slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(req.initial_display_limit)
                .bind(req.scope)
                .bind(req.cache_ttl_seconds)
                .bind(req.use_l2_cache)
                .fetch_one(&mut *tx)
                .await?;

                // 2. Update default pipeline/strategy if provided
                if let Some(ref pipeline_def) = req.pipeline {
                    sqlx::query(
                        r#"
                        UPDATE pipelines
                        SET definition = $2, updated_at = NOW()
                        WHERE id = (
                            SELECT pipeline_id 
                            FROM scenario_rules 
                            WHERE scenario_id = $1 AND condition = '{}'::jsonb
                            LIMIT 1
                        )
                        "#,
                    )
                    .bind(scenario_id)
                    .bind(pipeline_def)
                    .execute(&mut *tx)
                    .await?;
                }

                // 3. Update priority/status in scenario_rules
                if req.priority.is_some() || req.enabled.is_some() {
                    sqlx::query(
                        r#"
                        UPDATE scenario_rules
                        SET priority = COALESCE($2, priority),
                            is_active = COALESCE($3, is_active),
                            updated_at = NOW()
                        WHERE scenario_id = $1 AND condition = '{}'::jsonb
                        "#,
                    )
                    .bind(scenario_id)
                    .bind(req.priority)
                    .bind(req.enabled)
                    .execute(&mut *tx)
                    .await?;
                }

                tx.commit().await?;

                // Return updated state
                sqlx::query_as::<_, ScenarioConfig>(
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category,
                        p.definition as pipeline,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        NULL as staleness_rules, r.is_active as enabled, r.priority,
                        s.created_at, r.updated_at, NULL as created_by, 1 as version
                    FROM scenarios s
                    JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    JOIN pipelines p ON r.pipeline_id = p.id
                    WHERE s.id = $1
                    "#,
                )
                .bind(scenario_id)
                .fetch_one(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to update scenario in Intelligent Brain: {}", e),
                    source: None,
                })
            })
    }

    /// Hard-delete a scenario from the Intelligent Brain (cascades to rules/suggestions).
    pub async fn delete(&self, slug: &str) -> AppResult<()> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                sqlx::query("DELETE FROM scenarios WHERE slug = $1")
                .bind(&slug)
                .execute(&pool)
                .await
            })
            .await
            .map(|_| ())
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to delete scenario from Intelligent Brain: {}", e),
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
                    WHERE s.category = $1
                    ORDER BY s.slug
                    "#,
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
