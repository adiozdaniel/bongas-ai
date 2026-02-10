use anyhow::Result;
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

    // TODO: Phase 1+ initialization will go here

    Ok(())
}
