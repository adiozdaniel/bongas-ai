//! Phase 2: Regional Pulse Worker
//!
//! Scrapes regional news and events, classifies them via HiveMind,
//! and caches the semantic vectors in Redis for real-time boosting.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};
use anyhow::Result;

use crate::cache::CacheManager;
use crate::engine::intelligence::ai::hive_mind::service::{HiveMindConnector, PulseKind};

pub struct RegionalPulseWorker {
    hive_mind: Arc<HiveMindConnector>,
    cache_manager: Arc<CacheManager>,
    scrape_interval: Duration,
}

impl RegionalPulseWorker {
    pub fn new(
        hive_mind: Arc<HiveMindConnector>,
        cache_manager: Arc<CacheManager>,
        scrape_interval: Duration,
    ) -> Self {
        Self {
            hive_mind,
            cache_manager,
            scrape_interval,
        }
    }

    /// Start the background scraping and classification loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!(
            interval_mins = self.scrape_interval.as_secs() / 60,
            "Regional Pulse Worker started"
        );

        let mut ticker = interval(self.scrape_interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_scrape_cycle().await {
                        error!(error = %e, "Regional Pulse scrape cycle failed");
                    }
                }
                res = shutdown_rx.recv() => {
                    if res.is_ok() {
                        info!("Regional Pulse Worker shutting down...");
                        break;
                    }
                }
            }
        }
    }

    /// Perform a single scrape and cache cycle.
    async fn run_scrape_cycle(&self) -> Result<()> {
        debug!("Running regional pulse scrape cycle...");

        // Mock locations to scrape
        let locations = vec!["Nairobi", "Lagos", "Johannesburg", "London", "New York"];

        for location in locations {
            // 1. Scrape Headline (Mocked)
            let headline = self.mock_scrape_headline(location).await;
            
            // 2. Classify via HiveMind
            let classification = self.hive_mind.classify_pulse(&headline, location).await?;

            // 3. Cache in Redis if it's a Physical Event
            if classification.kind == PulseKind::PhysicalEvent && classification.confidence > 0.8 {
                let key = format!("pulse:{}", location.to_lowercase());
                self.cache_manager.set_with_ttl(
                    &key,
                    &classification,
                    Duration::from_secs(86400), // 24 hour TTL for regional pulses
                    "regional_pulse",
                    None,
                    None,
                ).await?;
                
                info!(location, theme = %classification.theme, "Cached regional physical pulse");
            }
        }

        Ok(())
    }

    async fn mock_scrape_headline(&self, location: &str) -> String {
        match location {
            "Nairobi" => "Sudden floods reported in Westlands area after heavy storm".to_string(),
            "London" => "Summer Jazz Festival kicks off in Hyde Park this weekend".to_string(),
            _ => format!("Standard daily activities in {}", location),
        }
    }
}
