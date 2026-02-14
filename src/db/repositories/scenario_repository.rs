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
