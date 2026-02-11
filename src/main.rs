use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
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

    // Create BongasEngine
    let model_dir = settings.ml.model_path.to_str().unwrap_or("models/onnx");
    let engine = BongasEngine::new(db_pool, clickhouse, &settings.redis.url, model_dir).await?;

    // Load scenarios from database
    let scenario_count = engine.reload_scenarios().await?;
    info!(scenarios = scenario_count, "Scenarios loaded");

    // Start Kafka consumers
    if std::env::var("KAFKA_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true" {
        engine.start_kafka_consumers(&settings.kafka.brokers).await?;
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
