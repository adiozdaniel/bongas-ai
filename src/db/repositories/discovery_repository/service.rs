//! Discovery configuration repository with Netflix-grade resilience patterns.

use std::sync::Arc;
use crate::resilience::ResilienceMetricsCollector;
use crate::db::DiscoveryConfig;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for managing device-specific discovery configurations.
pub struct DiscoveryConfigRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl DiscoveryConfigRepository {
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

    /// Upsert a discovery configuration.
    pub async fn upsert(
        &self, 
        config: DiscoveryConfig
    ) -> AppResult<DiscoveryConfig> {
        let start_time = std::time::Instant::now();
        let result: AppResult<DiscoveryConfig> = self.pool
            .execute(|pool| async move {
                let row: DiscoveryConfig = sqlx::query_as(
                    r#"
                    INSERT INTO bongas.discovery_configs 
                        (device_type, initial_batch_size, continuation_batch_size, prewarm_lookahead, ghost_ttl_seconds, cache_ttl_seconds, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, NOW())
                    ON CONFLICT (device_type) DO UPDATE
                    SET initial_batch_size = EXCLUDED.initial_batch_size,
                        continuation_batch_size = EXCLUDED.continuation_batch_size,
                        prewarm_lookahead = EXCLUDED.prewarm_lookahead,
                        ghost_ttl_seconds = EXCLUDED.ghost_ttl_seconds,
                        cache_ttl_seconds = EXCLUDED.cache_ttl_seconds,
                        updated_at = NOW()
                    RETURNING device_type, initial_batch_size, continuation_batch_size, prewarm_lookahead, ghost_ttl_seconds, cache_ttl_seconds, updated_at
                    "#
                )
                .bind(&config.device_type)
                .bind(config.initial_batch_size)
                .bind(config.continuation_batch_size)
                .bind(config.prewarm_lookahead)
                .bind(config.ghost_ttl_seconds)
                .bind(config.cache_ttl_seconds)
                .fetch_one(&pool)
                .await?;

                Ok(row)
            })
            .await;

        self.record_metrics("discovery_config_upsert", start_time, result.is_ok());

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to upsert discovery config: {}", e),
                source: None,
            })
        })
    }

    pub async fn find_all(&self) -> AppResult<Vec<DiscoveryConfig>> {
        let start_time = std::time::Instant::now();
        let result: AppResult<Vec<DiscoveryConfig>> = self.pool
            .execute(|pool| async move {
                let rows: Vec<DiscoveryConfig> = sqlx::query_as(
                    "SELECT device_type, initial_batch_size, continuation_batch_size, prewarm_lookahead, ghost_ttl_seconds, cache_ttl_seconds, updated_at FROM bongas.discovery_configs"
                )
                .fetch_all(&pool)
                .await?;

                Ok(rows)
            })
            .await;

        self.record_metrics("discovery_config_find_all", start_time, result.is_ok());

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to fetch discovery configs: {}", e),
                source: None,
            })
        })
    }

    pub async fn find_by_device(&self, device_type: &str) -> AppResult<Option<DiscoveryConfig>> {
        let device = device_type.to_string();
        let device_log = device_type.to_string();
        let start_time = std::time::Instant::now();

        let result: AppResult<Option<DiscoveryConfig>> = self.pool
            .execute(|pool| async move {
                let row: Option<DiscoveryConfig> = sqlx::query_as(
                    "SELECT device_type, initial_batch_size, continuation_batch_size, prewarm_lookahead, ghost_ttl_seconds, cache_ttl_seconds, updated_at FROM bongas.discovery_configs WHERE device_type = $1"
                )
                .bind(&device)
                .fetch_optional(&pool)
                .await?;

                Ok(row)
            })
            .await;

        self.record_metrics("discovery_config_find_by_device", start_time, result.is_ok());

        result.map_err(|e| {
            AppError::Postgres(PostgresError::Query {
                message: format!("Failed to fetch discovery config for {}: {}", device_log, e),
                source: None,
            })
        })
    }

    /// Internal helper to record performance metrics.
    fn record_metrics(&self, operation: &str, start_time: std::time::Instant, success: bool) {
        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create(operation);
        metrics.latency.record_duration(duration);
        if success {
            metrics.successes.increment();
        } else {
            metrics.failures.increment();
        }
    }
}
