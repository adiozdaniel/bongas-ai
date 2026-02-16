use std::sync::Arc;
use sqlx::PgPool;
use crate::AppConfig;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::middlewares::metrics::MetricsCollector;

/// Dependencies required to bootstrap the BongasEngine
pub struct EngineDependencies {
    pub config: Arc<AppConfig>,
    pub db_pool: PgPool,
    pub circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    pub metrics_collector: Arc<MetricsCollector>,
}

impl EngineDependencies {
    pub fn new(
        config: Arc<AppConfig>,
        db_pool: PgPool,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            config,
            db_pool,
            circuit_breaker_registry,
            metrics_collector,
        }
    }
}
