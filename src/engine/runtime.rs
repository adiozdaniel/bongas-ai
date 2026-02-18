use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{info, error};

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
        
        // 1. Load config (Sequential - foundational)
        info!("Loading configuration...");
        let config = Arc::new(
            ConfigLoader::new()
                .with_defaults()
                .with_env()
                .load()
                .context("Failed to load configuration")?
        );

        // 2. Initialize telemetry (Sequential - foundational for logs)
        initialize_telemetry(&TelemetryConfig::default())
            .context("Failed to initialize telemetry")?;
        info!("Telemetry initialized");

        // 3. Concurrently initialize independent resources
        info!("Initializing core resources concurrently...");
        
        let cb_observer = Arc::new(CompositeObserver::new(vec![Arc::new(TracingObserver)]));
        let circuit_breaker_registry = Arc::new(CircuitBreakerRegistry::with_observer(cb_observer));
        let metrics_collector = Arc::new(MetricsCollector::new());

        let db_config = config.database.clone();
        let redis_url = config.redis.url.clone();

        // Fire off connections in parallel
        let db_pool_fut = tokio::spawn(async move {
            info!("Establishing connection to PostgreSQL...");
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(db_config.max_connections)
                .acquire_timeout(std::time::Duration::from_secs(db_config.connection_timeout))
                .connect(db_config.url.as_ref().unwrap())
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to connect to PostgreSQL");
                    e
                })
        });

        let redis_client_fut = tokio::spawn(async move {
            info!("Establishing connection to Redis...");
            let client = redis::Client::open(redis_url.as_str())
                .map_err(|e| {
                    error!(error = %e, "Failed to connect to Redis");
                    e
                });
            client
        });

        // Wait for foundational connections
        let (db_pool_res, redis_client_res) = tokio::join!(db_pool_fut, redis_client_fut);
        
        let db_pool = db_pool_res.context("Postgres join error")?
            .context("Failed to connect to database")?;
        info!("Established PostgreSQL connection pool");

        let _redis_client = Arc::new(redis_client_res.context("Redis join error")?
            .context("Failed to create Redis client")?);
        info!("Established Redis connection");

        // 4. Bootstrap engine
        info!("Bootstrapping BongasEngine...");
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

        info!("Setting up API router...");
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

        info!(address = %bind_addr, "BongasRuntime ready! HTTP server starting");

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
