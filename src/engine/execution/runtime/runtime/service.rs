use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::{ConfigLoader, AppConfig, initialize_telemetry, TelemetryConfig};
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::engine::coordination::service::BongasEngine;
use crate::engine::config::models::EngineDependencies;
use crate::api::create_router;
use crate::middlewares::metrics::MetricsCollector;

/// High-level application orchestrator
pub struct BongasRuntime {
    config: Arc<AppConfig>,
    engine: Arc<BongasEngine>,
    start_time: Instant,
}

impl BongasRuntime {
    /// Initialize and bootstrap the application.
    pub async fn init() -> Result<Self> {
        let start_time = Instant::now();

        // 1. Load Configuration
        let config = Arc::new(ConfigLoader::new().load()?);

        // 2. Initialize Telemetry
        let telemetry_config = TelemetryConfig::builder()
            .service_name("bongas-ai".to_string())
            .environment(config.server.environment.clone())
            .default_level(crate::telemetry::LogLevel::Info)
            .build()
            .map_err(|e| anyhow::anyhow!("Telemetry config error: {}", e))?;

        if let Err(e) = initialize_telemetry(&telemetry_config) {
            warn!("Telemetry initialization warning: {}", e);
        }

        info!("Bongas-AI Symphony 2.0 initializing...");

        // 3. Initialize Resilience Layer
        let breaker_registry = Arc::new(CircuitBreakerRegistry::new());
        let metrics_collector = Arc::new(MetricsCollector::new());

        // 4. Initialize Database
        let db_url = config.database.url.as_deref()
            .ok_or_else(|| anyhow::anyhow!("DATABASE_URL not configured"))?;
            
        let db_pool = sqlx::PgPool::connect(db_url).await
            .context("Failed to connect to database")?;

        // 5. Initialize Core Engine (The Brain)
        let deps = EngineDependencies {
            config: config.clone(),
            db_pool,
            circuit_breaker_registry: breaker_registry.clone(),
            metrics_collector: metrics_collector.clone(),
        };

        let engine = BongasEngine::bootstrap(deps).await
            .context("Failed to bootstrap BongasEngine")?;

        Ok(Self {
            config,
            engine,
            start_time,
        })
    }

    /// Run the application and start the HTTP server.
    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&addr).await
            .context(format!("Failed to bind to {}", addr))?;

        let metrics_collector = Arc::new(MetricsCollector::new());
        let breaker_registry = Arc::new(CircuitBreakerRegistry::new());
        
        // We need a redis client for the router
        let redis_client = Arc::new(redis::Client::open(self.config.redis.url.clone())?);

        let app = create_router(
            self.engine.clone(),
            self.config.clone(),
            redis_client,
            metrics_collector,
            breaker_registry,
            Arc::new(self.start_time),
        );

        info!("🎼 Symphony 2.0 serving at http://{}", addr);
        
        axum::serve(listener, app)
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .context("Server execution failed")?;

        Ok(())
    }

    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
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
