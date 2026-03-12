use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::{ConfigLoader, AppConfig, initialize_telemetry, TelemetryConfig};
use crate::engine::coordination::{BongasEngine, DiscoverySymphony};
use crate::api::create_router;
use crate::api::ConnectionTracker;

/// High-level application orchestrator
pub struct BongasRuntime {
    config: Arc<AppConfig>,
    engine: Arc<BongasEngine>,
}

impl BongasRuntime {
    /// Initialize and bootstrap the application using the DiscoverySymphony factory.
    pub async fn init() -> Result<Self> {
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

        info!("🎼 Bongas-AI Symphony 2.0 Bootstrapping...");

        // 3. The Grand Factory: Unified Bootstrap & DI
        let symphony = DiscoverySymphony::new(config.clone());
        let engine = symphony.assemble().await
            .context("Failed to assemble the Bongas-AI Engine Symphony")?;

        Ok(Self {
            config,
            engine,
        })
    }

    /// Run the application and start the HTTP server.
    pub async fn run(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&addr).await
            .context(format!("Failed to bind to {}", addr))?;

        // Extract shared state from engine for the router
        let redis_client = Arc::new(redis::Client::open(self.config.redis.url.clone())?);
        let tracker = Arc::new(ConnectionTracker::new(10)); // Default limit

        let app = create_router(
            self.engine.clone(),
            self.config.clone(),
            redis_client,
            self.engine.resilience_metrics.clone(),
            self.engine.circuit_breaker_registry(),
            tracker,
        );

        info!("🚀 Symphony 2.0 serving at http://{}", addr);
        
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
