//! Phase 2: Regional Pulse Worker
//!
//! Scrapes regional news and events for semantic ranking boosts.

use tokio::sync::broadcast;
use tracing::info;

pub struct RegionalPulseWorker;

impl RegionalPulseWorker {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(self, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("Regional Pulse Worker started");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Regional Pulse Worker shutting down...");
                    break;
                }
            }
        }
    }
}
