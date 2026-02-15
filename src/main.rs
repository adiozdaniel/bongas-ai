//! BONGAS-AI service entrypoint with Netflix-grade initialization and orchestration.
//!
//! Implements the main application lifecycle management including configuration loading,
//! circuit breaker registry setup, database connectivity, Redis caching, metrics collection,
//! engine initialization, cache warming, and HTTP server startup with graceful shutdown.

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::signal;
use tokio::net::TcpListener;
use tracing::{info, warn};

use bongas_ai::{
    ConfigLoader, AppConfig,
    initialize_telemetry, TelemetryConfig,
};
use bongas_ai::circuit_breaker::{
    CircuitBreakerRegistry, CompositeObserver, TracingObserver,
};
use bongas_ai::cache::{CacheConfig, CacheManager, CacheWarmer};
use bongas_ai::engine::BongasEngine;
use bongas_ai::api::create_router;
use bongas_ai::middlewares::metrics::MetricsCollector;

/// Application state shared across HTTP handlers
pub struct AppState {
    pub engine: Arc<BongasEngine>,
    pub config: Arc<AppConfig>,
    pub circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    pub metrics_collector: Arc<MetricsCollector>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // ═══════════════════════════════════════════════════════════════════════════
    // 1. LOAD CONFIGURATION
    // ═══════════════════════════════════════════════════════════════════════════
    let config = Arc::new(
        ConfigLoader::new()
            .with_defaults()
            .with_env()
            .load()
            .context("Failed to load configuration")?
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 2. INITIALIZE TELEMETRY (logging, tracing, metrics)
    // ═══════════════════════════════════════════════════════════════════════════
    let _telemetry_guard = initialize_telemetry(&TelemetryConfig::default())
        .context("Failed to initialize telemetry")?;

    info!(
        config_source = "composite",
        services = ?config.enabled_services(),
        "Configuration loaded successfully"
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 3. SETUP CIRCUIT BREAKER REGISTRY WITH OBSERVERS
    // ═══════════════════════════════════════════════════════════════════════════
    let circuit_breaker_observer = Arc::new(CompositeObserver::new(vec![
        Arc::new(TracingObserver),
        // Add AlertObserver here when configured:
        // Arc::new(AlertObserver::new(AlertConfig::from(&config))),
    ]));

    let circuit_breaker_registry = Arc::new(
        CircuitBreakerRegistry::with_observer(circuit_breaker_observer)
    );

    info!(
        observer_count = 1,
        "Circuit breaker registry initialized with observers"
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 4. INITIALIZE DATABASE POOL
    // ═══════════════════════════════════════════════════════════════════════════
    let database_url = config.database.url.as_ref()
        .context("DATABASE_URL not configured")?;

    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(config.database.connection_timeout))
        .connect(database_url)
        .await
        .context("Failed to connect to database")?;

    info!(
        max_connections = config.database.max_connections,
        "Database pool initialized"
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 5. CREATE CACHE CONFIGURATION
    // ═══════════════════════════════════════════════════════════════════════════
    let cache_config = CacheConfig::default();

    // ═══════════════════════════════════════════════════════════════════════════
    // 6. INITIALIZE BONGAS ENGINE
    // ═══════════════════════════════════════════════════════════════════════════
    let model_path = config.ml.model_path.to_str()
        .context("Invalid model path")?;

    let engine = BongasEngine::new(
        db_pool,
        &config.redis.url,
        model_path,
        config.security.clone(),
        cache_config.clone(),
        circuit_breaker_registry.clone(),
    )
    .await
    .context("Failed to initialize BongasEngine")?;

    info!("BongasEngine initialized successfully");

    // ═══════════════════════════════════════════════════════════════════════════
    // 7. LOAD SCENARIOS FROM DATABASE
    // ═══════════════════════════════════════════════════════════════════════════
    let scenario_count = engine.reload_scenarios().await
        .context("Failed to load scenarios")?;

    info!(scenario_count = scenario_count, "Scenarios loaded from database");

    // ═══════════════════════════════════════════════════════════════════════════
    // 8. START CACHE WARMER (background task)
    // ═══════════════════════════════════════════════════════════════════════════
    if cache_config.warming_enabled {
        let cache_manager = Arc::new(
            CacheManager::new(&config.redis.url, cache_config.clone())
                .await
                .context("Failed to create CacheManager for warming")?
        );

        let warmer = Arc::new(CacheWarmer::new(
            cache_manager,
            cache_config.warm_scenarios.clone(),
            cache_config.warming_interval,
        ));

        let warmer_handle = warmer.clone();
        tokio::spawn(async move {
            warmer_handle.start().await;
        });

        info!(
            scenarios = ?cache_config.warm_scenarios,
            interval_secs = cache_config.warming_interval.as_secs(),
            "Cache warmer started"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 9. START KAFKA CONSUMERS (if configured)
    // ═══════════════════════════════════════════════════════════════════════════
    if !config.kafka.brokers.is_empty() {
        match engine.start_kafka_consumers(&config.kafka.brokers).await {
            Ok(()) => info!(brokers = ?config.kafka.brokers, "Kafka consumers started"),
            Err(e) => warn!(error = %e, "Failed to start Kafka consumers, continuing without"),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 10. CREATE METRICS COLLECTOR
    // ═══════════════════════════════════════════════════════════════════════════
    let metrics_collector = Arc::new(MetricsCollector::new());

    // ═══════════════════════════════════════════════════════════════════════════
    // 11. CREATE REDIS CLIENT FOR RATE LIMITING
    // ═══════════════════════════════════════════════════════════════════════════
    let redis_client = Arc::new(
        redis::Client::open(config.redis.url.as_str())
            .context("Failed to create Redis client")?
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 12. CREATE HTTP ROUTER
    // ═══════════════════════════════════════════════════════════════════════════
    let router = create_router(
        engine.clone(),
        redis_client,
        metrics_collector.clone(),
        circuit_breaker_registry.clone(),
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 13. START HTTP SERVER
    // ═══════════════════════════════════════════════════════════════════════════
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .context(format!("Failed to bind to {}", bind_addr))?;

    info!(
        address = %bind_addr,
        "HTTP server starting"
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // 14. GRACEFUL SHUTDOWN HANDLING
    // ═══════════════════════════════════════════════════════════════════════════
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server error")?;

    info!("Server shutdown complete");

    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
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
