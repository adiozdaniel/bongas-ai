  //! Cache warming strategies.

  use crate::cache::manager::CacheManager;
  use anyhow::Result;
  use std::sync::Arc;
  use tokio::time::{interval, Duration};

  /// Cache warming service for pre-loading hot data.
  pub struct CacheWarmer {
      cache_manager: Arc<CacheManager>,
      scenarios: Vec<String>,
      interval: Duration,
      shutdown_rx: tokio::sync::broadcast::Receiver<()>,
  }

  impl CacheWarmer {
      pub fn new(
          cache_manager: Arc<CacheManager>,
          scenarios: Vec<String>,
          warming_interval: Duration,
          shutdown_rx: tokio::sync::broadcast::Receiver<()>,
      ) -> Self {
          Self {
              cache_manager,
              scenarios,
              interval: warming_interval,
              shutdown_rx,
          }
      }

      /// Start the cache warming background task.
      pub async fn start(self: Arc<Self>) {
          let mut ticker = interval(self.interval);
          let mut shutdown_rx = self.shutdown_rx.resubscribe();

          loop {
              tokio::select! {
                  _ = ticker.tick() => {
                      tracing::info!(
                          scenarios = ?self.scenarios,
                          "Starting cache warming cycle"
                      );

                      if let Err(e) = self.warm_caches().await {
                          tracing::error!(error = %e, "Cache warming failed");
                      }
                  }
                  _ = shutdown_rx.recv() => {
                      tracing::info!("Cache warmer shutting down...");
                      break;
                  }
              }
          }
      }

      /// Warm caches for all configured scenarios.
      async fn warm_caches(&self) -> Result<()> {
          for scenario in &self.scenarios {
              self.warm_scenario(scenario).await?;
          }
          Ok(())
      }

      /// Warm cache for a specific scenario.
      async fn warm_scenario(&self, scenario: &str) -> Result<()> {
          tracing::debug!(scenario = scenario, "Warming cache for scenario");

          // Pre-warm with scenario metadata to ensure cache is hot
          let cache_key = format!("scenario:warm:{}", scenario);
          let warm_marker = WarmMarker {
              scenario: scenario.to_string(),
              warmed_at: chrono::Utc::now().timestamp(),
          };

          self.cache_manager.set(&cache_key, &warm_marker).await?;

          tracing::debug!(
              scenario = scenario,
              cache_key = %cache_key,
              "Scenario cache warmed"
          );

          Ok(())
      }

      /// Check if a scenario was recently warmed.
      pub async fn is_warm(&self, scenario: &str) -> bool {
          let cache_key = format!("scenario:warm:{}", scenario);
          self.cache_manager
              .get::<WarmMarker>(&cache_key)
              .await
              .ok()
              .flatten()
              .is_some()
      }
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  struct WarmMarker {
      scenario: String,
      warmed_at: i64,
  }

