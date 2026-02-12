use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod engine;
mod scenarios;
mod ml;
mod experiments;
mod db;
mod cache;
mod kafka;
mod analytics;
mod security;
mod config;
mod pipeline;
mod middlewares;

use engine::BongasEngine;
use analytics::ClickHouseClient;
use config::settings::Settings;
use security::manager::SecurityManager;
use middlewares::metrics::MetricsCollector;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bongas_ai=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting BONGAS-AI v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let settings = Settings::load()?;
    info!("Configuration loaded");

    // Initialize security validation (8-layer security)
    let security_manager = if !settings.security.license_key.is_empty() {
        info!("🔒 Initializing security validation...");
        let manager = SecurityManager::new(&settings.security).await?;
        
        match manager.validate_license().await {
            Ok(()) => {
                info!("✅ All 8 security layers passed");
                Some(Arc::new(manager))
            }
            Err(e) => {
                error!("❌ Security validation failed: {}", e);
                return Err(e);
            }
        }
    } else {
        // In production, license key is mandatory
        #[cfg(not(debug_assertions))]
        {
            error!("❌ License key required in production mode");
            return Err(anyhow::anyhow!("License key required in production"));
        }
        
        // In development, allow running without license
        #[cfg(debug_assertions)]
        {
            warn!("⚠️ Security validation skipped (development mode - no license key)");
            None
        }
    };

    // Initialize database pool
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.url)
        .await?;

    info!("Database connection established");

    // Initialize ClickHouse client
    let clickhouse = ClickHouseClient::new(&settings.clickhouse);
    info!("ClickHouse client initialized");

    // Create Redis client for API layer
    let redis_client = Arc::new(
        redis::Client::open(settings.redis.url.as_str())?
    );

    // Create metrics collector
    let metrics_collector = Arc::new(MetricsCollector::new());

    // Create BongasEngine with security manager
    let model_dir = settings.ml.model_path.to_str().unwrap_or("models/onnx");
    let engine = BongasEngine::new(
        db_pool,
        clickhouse,
        &settings.redis.url,
        model_dir,
        security_manager,
    ).await?;

    // Load scenarios from database
    let scenario_count = engine.reload_scenarios().await?;
    info!(scenarios = scenario_count, "Scenarios loaded");

    // Load experiments from database configuration
    let experiment_count = engine.load_experiments_from_db().await?;
    info!(experiments = experiment_count, "Experiments loaded from database");

    // Start Kafka consumers
    if std::env::var("KAFKA_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true" {
        engine.start_kafka_consumers(&settings.kafka.brokers).await?;
    }

    // Start cache warming for popular scenarios
    if std::env::var("CACHE_WARMING_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true" {
        let warm_scenarios = settings.cache.warm_scenarios.clone().unwrap_or_else(|| vec![
            "personalized_home".to_string(),
            "continue_watching".to_string(),
            "trending_now".to_string(),
            "live_tv".to_string(),
        ]);
        
        // Validate scenarios exist before starting cache warming
        let available_scenarios = engine.list_scenarios().await;
        let mut valid_scenarios = Vec::new();
        
        for scenario in &warm_scenarios {
            if available_scenarios.contains(scenario) {
                valid_scenarios.push(scenario.clone());
            } else {
                warn!("Scenario '{}' not found, skipping cache warming", scenario);
            }
        }

        if !valid_scenarios.is_empty() {
            let warm_interval = settings.cache.warming_interval_minutes.unwrap_or(30);
            let scenario_count = valid_scenarios.len();
            engine.clone().start_cache_warming(valid_scenarios, warm_interval);
            info!(
                scenarios = scenario_count,
                interval_minutes = warm_interval,
                "Cache warming started for {} scenarios",
                scenario_count
            );
        } else {
            warn!("No valid scenarios found for cache warming, skipping");
        }
    }

    // Create API router
    let app = api::create_router(engine.clone(), redis_client, metrics_collector);

    // Start server
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener: TcpListener = TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;
    info!("Server listening on {}", local_addr);

    // Graceful shutdown handling
    let engine_shutdown = engine.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        info!("Shutdown signal received, cleaning up...");
        engine_shutdown.shutdown_kafka_consumers().await;
    });

    axum::serve(listener, app).await?;

    Ok(())
}
