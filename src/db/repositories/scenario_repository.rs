//! Scenario repository with Netflix-grade resilience patterns.
//!
//! Provides database access for scenario configurations with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::resilience::ResilienceMetricsCollector;
use crate::db::models::{Scenario, ScenarioWithStrategy, PipelineDefinition};
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};
use crate::api::models::scenario::{CreateScenarioRequest, UpdateScenarioRequest};

#[derive(sqlx::FromRow)]
struct FlatScenarioRow {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub target_kpi: String,
    pub initial_display_limit: i32,
    pub scope: serde_json::Value,
    pub cache_ttl_seconds: i32,
    pub use_l2_cache: bool,
    pub created_at: DateTime<Utc>,
    pub pipeline: serde_json::Value,
    pub is_active: bool,
    pub priority: i32,
}

impl FlatScenarioRow {
    fn into_scenario_with_strategy(self) -> ScenarioWithStrategy {
        ScenarioWithStrategy {
            scenario: Scenario {
                id: self.id,
                slug: self.slug,
                name: self.name,
                description: self.description,
                category: self.category,
                target_kpi: self.target_kpi,
                initial_display_limit: self.initial_display_limit,
                scope: self.scope,
                cache_ttl_seconds: self.cache_ttl_seconds,
                use_l2_cache: self.use_l2_cache,
                created_at: self.created_at,
            },
            pipeline: serde_json::from_value(self.pipeline).unwrap_or_else(|_| PipelineDefinition { stages: vec![], fallback_stages: None }),
            is_active: self.is_active,
            priority: self.priority,
        }
    }
}

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
    pub async fn create(&self, req: CreateScenarioRequest) -> AppResult<ScenarioWithStrategy> {
        self.pool
            .execute(|pool| async move {
                let mut tx = pool.begin().await?;

                // 1. Insert into scenarios
                let scenario_id: i32 = sqlx::query_scalar(
                    r#"
                    INSERT INTO scenarios (
                        slug, name, description, category, target_kpi,
                        initial_display_limit, scope,
                        cache_ttl_seconds, use_l2_cache
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    RETURNING id
                    "#,
                )
                .bind(&req.slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(req.target_kpi.as_deref().unwrap_or("retention"))
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

                self.internal_find_with_strategy_by_id(scenario_id, &pool).await
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
    pub async fn update(&self, slug: &str, req: UpdateScenarioRequest) -> AppResult<ScenarioWithStrategy> {
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
                        target_kpi = COALESCE($5, target_kpi),
                        initial_display_limit = COALESCE($6, initial_display_limit),
                        scope = COALESCE($7, scope),
                        cache_ttl_seconds = COALESCE($8, cache_ttl_seconds),
                        use_l2_cache = COALESCE($9, use_l2_cache)
                    WHERE slug = $1
                    RETURNING id
                    "#,
                )
                .bind(&slug)
                .bind(&req.name)
                .bind(&req.description)
                .bind(&req.category)
                .bind(&req.target_kpi)
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

                self.internal_find_with_strategy_by_id(scenario_id, &pool).await
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

    /// Load all active scenarios and their default strategies.
    pub async fn find_all_active(&self) -> AppResult<Vec<ScenarioWithStrategy>> {
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                let rows: Vec<FlatScenarioRow> = sqlx::query_as(
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category, s.target_kpi,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        s.created_at,
                        p.definition as pipeline,
                        r.is_active, r.priority
                    FROM scenarios s
                    JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    JOIN pipelines p ON r.pipeline_id = p.id
                    WHERE r.is_active = true
                    ORDER BY r.priority DESC, s.slug
                    "#,
                )
                .fetch_all(&pool)
                .await?;

                let res = rows.into_iter().map(|row| row.into_scenario_with_strategy()).collect();
                Ok(res)
            })
            .await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create("scenario");
        metrics.latency.record_duration(duration);
        
        match result {
            Ok(scenarios) => {
                metrics.successes.increment();
                Ok(scenarios)
            }
            Err(e) => {
                metrics.failures.increment();
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch active scenarios: {}", e),
                    source: None,
                }))
            }
        }
    }

    /// Find scenario and its strategy by slug.
    pub async fn find_by_slug(&self, slug: &str) -> AppResult<Option<ScenarioWithStrategy>> {
        let slug = slug.to_string();
        self.pool
            .execute(|pool| async move {
                let row: Option<FlatScenarioRow> = sqlx::query_as(
                    r#"
                    SELECT 
                        s.id, s.slug, s.name, s.description, s.category, s.target_kpi,
                        s.initial_display_limit, s.scope, 
                        s.cache_ttl_seconds, s.use_l2_cache,
                        s.created_at,
                        p.definition as pipeline,
                        r.is_active, r.priority
                    FROM scenarios s
                    JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
                    JOIN pipelines p ON r.pipeline_id = p.id
                    WHERE s.slug = $1
                    "#,
                )
                .bind(&slug)
                .fetch_optional(&pool)
                .await?;

                Ok(row.map(|r| r.into_scenario_with_strategy()))
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch scenario by slug: {}", e),
                    source: None,
                })
            })
    }

    /// Internal helper to fetch strategy by ID
    async fn internal_find_with_strategy_by_id(&self, id: i32, pool: &sqlx::PgPool) -> sqlx::Result<ScenarioWithStrategy> {
        let row: FlatScenarioRow = sqlx::query_as(
            r#"
            SELECT 
                s.id, s.slug, s.name, s.description, s.category, s.target_kpi,
                s.initial_display_limit, s.scope, 
                s.cache_ttl_seconds, s.use_l2_cache,
                s.created_at,
                p.definition as pipeline,
                r.is_active, r.priority
            FROM scenarios s
            JOIN scenario_rules r ON s.id = r.scenario_id AND r.condition = '{}'::jsonb
            JOIN pipelines p ON r.pipeline_id = p.id
            WHERE s.id = $1
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(row.into_scenario_with_strategy())
    }
}
