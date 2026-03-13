//! Phase 2: Regional Pulse Worker
//!
//! Scrapes regional news and events for semantic ranking boosts.

use tokio::sync::broadcast;
use tracing::info;

pub struct RegionalPulseWorker;

impl Default for RegionalPulseWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionalPulseWorker {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(self, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("Regional Pulse Worker started");

        loop {
            tokio::select! {
                res = shutdown_rx.recv() => {
                    if res.is_ok() {
                        info!("Regional Pulse Worker shutting down...");
                        break;
                    }
                }
                // Add a periodic tick here in the future
                _ = tokio::time::sleep(std::time::Duration::from_secs(3600)) => {
                    info!("Regional Pulse Worker pulse check...");
                }
            }
        }
    }
}
