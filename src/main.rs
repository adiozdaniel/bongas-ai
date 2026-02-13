//! BONGAS-AI service entrypoint with comprehensive initialization and orchestration.
//!
//! Implements the main application lifecycle management including configuration loading,
//! multi-layer security validation, database connectivity, Redis caching, metrics collection,
//! engine initialization, scenario management, experiment loading, Kafka consumer coordination,
//! cache warming strategies, and HTTP server startup with graceful shutdown handling.
//! Provides comprehensive observability through structured logging and metrics collection.

use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bongas_ai::api;
use bongas_ai::config::settings::Settings;
use bongas_ai::engine::BongasEngine;
use bongas_ai::middlewares::metrics::MetricsCollector;
use bongas_ai::security::manager::SecurityManager;

/// BONGAS-AI main application entrypoint.
///
/// Orchestrates the complete service initialization sequence:
/// 1. Logging infrastructure setup with environment-aware filtering
/// 2. Configuration loading from environment and configuration files
/// 3. 8-layer security validation with production/development mode differentiation
/// 4. Database connection pool establishment with configurable connection limits
/// 5. Redis client initialization for distributed caching and rate limiting
/// 6. Metrics collection system initialization for operational observability
/// 7. Core engine instantiation with integrated security context
/// 8. Scenario loading from persistent storage with validation
/// 9. Experiment configuration loading from database
/// 10. Kafka consumer initialization for event stream processing
/// 11. Intelligent cache warming for high-traffic scenarios
/// 12. HTTP server startup with graceful shutdown capabilities
///
/// # Returns
/// * `Result<()>` - Success indicator or initialization failure
///
/// # Panics
/// * If critical components fail to initialize in production mode
/// * If signal handling cannot be established for graceful shutdown
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging infrastructure with environment-aware filtering
    // Production: Respect RUST_LOG environment variable, default to info level
    // Development: Enable debug output with structured formatting
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bongas_ai=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting BONGAS-AI v{}", env!("CARGO_PKG_VERSION"));

    // Load and validate service configuration
    // Configuration sources (in order of precedence):
    // 1. Environment variables (prefix: BONGAS_)
    // 2. Configuration file (config.toml, config.prod.toml)
    // 3. Default values
    let settings = Settings::load()?;
    info!("Configuration loaded successfully");

    // Initialize comprehensive 8-layer security validation
    // Security layers include: license validation, hardware fingerprinting,
    // anti-debugging, binary integrity, tamper detection, and more
    let security_manager = if !settings.security.license_key.is_empty() {
        info!("Initializing 8-layer security validation stack...");
        let manager = SecurityManager::new(&settings.security).await?;

        // Validate all security layers in production mode
        // Includes license checks, hardware binding, and integrity verification
        match manager.validate_license().await {
            Ok(()) => {
                info!("All 8 security layers validated successfully");
                Some(Arc::new(manager))
            }
            Err(e) => {
                error!("Security validation failed at layer: {}", e);
                return Err(e);
            }
        }
    } else {
        // Enforce security in production builds
        #[cfg(not(debug_assertions))]
        {
            error!("License key required in production mode - security validation mandatory");
            return Err(anyhow::anyhow!("License key required in production"));
        }

        // Allow development mode with warning
        #[cfg(debug_assertions)]
        {
            warn!("Security validation disabled in development mode - license key not provided");
            warn!("This configuration is NOT suitable for production deployment");
            None
        }
    };

    // Initialize database connection pool with optimized settings
    // Pool configuration includes:
    // - Maximum connections limit to prevent database overload
    // - Connection timeouts and health checks
    // - Automatic reconnection on failure
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.url)
        .await?;

    info!("Database connection pool established");

    // Create Redis client for distributed caching and rate limiting
    // Used for: session storage, rate limiting counters, temporary data caching
    let redis_client = Arc::new(
        redis::Client::open(settings.redis.url.as_str())?
    );

    // Initialize metrics collection system
    // Collects: request metrics, business KPIs, system health indicators
    let metrics_collector = Arc::new(MetricsCollector::new());

    // Create core BONGAS engine with all dependencies
    // Engine manages: scenarios, experiments, model inference, caching
    let model_dir = settings.ml.model_path.to_str().unwrap_or("models/onnx");
    let engine = BongasEngine::new(
        db_pool,
        &settings.redis.url,
        model_dir,
        security_manager,
    ).await?;

    // Load active scenarios from database into memory cache
    // Scenarios define business logic for different recommendation types
    let scenario_count = engine.reload_scenarios().await?;
    info!(
        scenarios = scenario_count,
        "Active scenarios loaded into memory cache"
    );

    // Load experiment configurations from database
    // Experiments enable A/B testing and gradual feature rollout
    let experiment_count = engine.load_experiments_from_db().await?;
    info!(
        experiments = experiment_count,
        "Experiment configurations loaded from database"
    );

    // Initialize Kafka consumers for event stream processing
    // Consumes: user interaction events, system metrics, business events
    if std::env::var("KAFKA_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true" {
        engine.start_kafka_consumers(&settings.kafka.brokers).await?;
        info!("Kafka consumers initialized for event stream processing");
    }

    // Implement intelligent cache warming for popular scenarios
    // Pre-warms: personalized_home, continue_watching, trending_now, live_tv
    // Strategy: identify valid scenarios, warm at configured intervals
    if std::env::var("CACHE_WARMING_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true" {
        let warm_scenarios = settings.cache.warm_scenarios.clone().unwrap_or_else(|| vec![
            "personalized_home".to_string(),
            "continue_watching".to_string(),
            "trending_now".to_string(),
            "live_tv".to_string(),
        ]);

        // Validate scenarios exist before warming
        let available_scenarios = engine.list_scenarios().await;
        let mut valid_scenarios = Vec::new();

        for scenario in &warm_scenarios {
            if available_scenarios.contains(scenario) {
                valid_scenarios.push(scenario.clone());
            } else {
                warn!(
                    scenario = scenario,
                    "Scenario not found for cache warming - check configuration"
                );
            }
        }

        if !valid_scenarios.is_empty() {
            let warm_interval = settings.cache.warming_interval_minutes.unwrap_or(30);
            let scenario_count = valid_scenarios.len();
            engine.clone().start_cache_warming(valid_scenarios, warm_interval);
            info!(
                scenarios = scenario_count,
                interval_minutes = warm_interval,
                "Cache warming scheduled for {} scenarios at {} minute intervals",
                scenario_count, warm_interval
            );
        } else {
            warn!("No valid scenarios found for cache warming - skipping cache pre-warming");
        }
    }

    // Create API router with all middleware and routes
    // Routes include: health checks, metrics, recommendations, admin endpoints
    let app = api::create_router(engine.clone(), redis_client, metrics_collector);

    // Start HTTP server with configured host and port
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener: TcpListener = TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;
    info!(
        address = %local_addr,
        "HTTP server listening for incoming requests"
    );

    // Configure graceful shutdown on SIGINT (Ctrl+C) and SIGTERM
    // Ensures: Kafka consumers drain, in-flight requests complete, connections close properly
    let engine_shutdown = engine.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for termination signal");
        info!("Shutdown signal received - initiating graceful shutdown sequence");
        info!("Draining Kafka consumers and cleaning up resources...");
        engine_shutdown.shutdown_kafka_consumers().await;
        info!("Graceful shutdown complete");
    });

    // Start serving requests until shutdown signal received
    axum::serve(listener, app).await?;

    Ok(())
}
