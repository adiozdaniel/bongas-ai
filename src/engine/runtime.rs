use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::info;

use crate::{ConfigLoader, AppConfig, initialize_telemetry, TelemetryConfig};
use crate::circuit_breaker::{CircuitBreakerRegistry, CompositeObserver, TracingObserver};
use crate::engine::BongasEngine;
use crate::engine::config::EngineDependencies;
use crate::api::create_router;
use crate::middlewares::metrics::MetricsCollector;

/// High-level application orchestrator
pub struct BongasRuntime {
    config: Arc<AppConfig>,
    engine: Arc<BongasEngine>,
    circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    metrics_collector: Arc<MetricsCollector>,
    start_time: Arc<Instant>,
}

impl BongasRuntime {
    /// Initialize the application and all its components
    pub async fn init() -> Result<Self> {
        let start_time = Arc::new(Instant::now());
        // 1. Load config
        let config = Arc::new(
            ConfigLoader::new()
                .with_defaults()
                .with_env()
                .load()
                .context("Failed to load configuration")?
        );

        // 2. Initialize telemetry
        initialize_telemetry(&TelemetryConfig::default())
            .context("Failed to initialize telemetry")?;

        info!("Telemetry initialized");

        // 3. Setup Circuit Breaker Registry
        let circuit_breaker_observer = Arc::new(CompositeObserver::new(vec![
            Arc::new(TracingObserver),
        ]));
        let circuit_breaker_registry = Arc::new(
            CircuitBreakerRegistry::with_observer(circuit_breaker_observer)
        );

        // 4. Database pool
        let database_url = config.database.url.as_ref()
            .context("DATABASE_URL not configured")?;

        let db_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.database.connection_timeout))
            .connect(database_url)
            .await
            .context("Failed to connect to database")?;

        // 5. Shared metrics
        let metrics_collector = Arc::new(MetricsCollector::new());

        // 6. Bootstrap engine
        let deps = EngineDependencies::new(
            config.clone(),
            db_pool,
            circuit_breaker_registry.clone(),
            metrics_collector.clone(),
        );
        let engine = BongasEngine::bootstrap(deps).await?;

        Ok(Self {
            config,
            engine,
            circuit_breaker_registry,
            metrics_collector,
            start_time,
        })
    }

    /// Run the application HTTP server
    pub async fn run(self) -> Result<()> {
        let redis_client = Arc::new(
            redis::Client::open(self.config.redis.url.as_str())
                .context("Failed to create Redis client")?
        );

        let router = create_router(
            self.engine.clone(),
            self.config.clone(),
            redis_client,
            self.metrics_collector.clone(),
            self.circuit_breaker_registry.clone(),
            self.start_time.clone(),
        );

        let bind_addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&bind_addr)
            .await
            .context(format!("Failed to bind to {}", bind_addr))?;

        info!(address = %bind_addr, "HTTP server starting");

        axum::serve(listener, router)
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .context("HTTP server error")?;

        info!("Server shutdown complete");
        Ok(())
    }

    async fn shutdown_signal() {
        use tokio::signal;
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
            _ = terminate => info!("Received SIGTERM, shutting down..."),
        }
    }
}
