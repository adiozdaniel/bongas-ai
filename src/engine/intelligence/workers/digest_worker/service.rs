//! Phase 7: Scheduled Intelligence (Digest Worker)
//!
//! Scans for inactive users, executes the email_digest scenario, and
//! dispatches retention emails through the Notification Hub.

use std::sync::{Arc, Weak};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use anyhow::Result;
use serde_json::json;

use crate::engine::coordination::service::BongasEngine;
use crate::engine::execution::core::execution_manager::ScenarioExecutionContext;
use crate::notification::models::{NotificationIntent, EmailPayload};

pub struct DigestWorker {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    interval: Duration,
}

impl DigestWorker {
    pub fn new(interval: Duration) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            interval,
        }
    }

    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(engine);
    }

    /// Start the background digest generation loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!(
            interval_mins = self.interval.as_secs() / 60,
            "Digest Worker started"
        );

        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_digest_cycle().await {
                        error!(error = %e, "Digest cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Digest Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn run_digest_cycle(&self) -> Result<()> {
        debug!("Running weekly digest cycle for inactive profiles...");

        let engine_arc: Option<Arc<BongasEngine>> = {
            let guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|w| w.upgrade())
        };

        let engine = match engine_arc {
            Some(e) => e,
            None => return Ok(()),
        };

        // 1. Identify inactive profiles
        // In a real environment, we'd query ClickHouse: 
        // SELECT profile_id FROM user_interactions GROUP BY profile_id HAVING max(created_at) < now() - 3 days
        let inactive_profiles = vec!["usr_123_dormant".to_string(), "usr_456_dormant".to_string()];

        for profile_id in inactive_profiles {
            // 2. Execute the 'email_digest' scenario
            let ctx = ScenarioExecutionContext {
                scenario_slug: "email_digest".to_string(),
                user_id: None,
                profile_id: Some(profile_id.clone()),
                maturity_rating: Some("18".to_string()),
                device_type: None,
                context_params: json!({}),
                limit: Some(5), // Top 5 items for the email
                request_id: None,
            };

            let result = engine.execute_scenario_with_stats_contextual(ctx).await;

            if let Ok((items, _stats)) = result {
                if items.is_empty() {
                    continue;
                }

                // 3. Package into EmailPayload with Dynamic Reasoning
                let mut email_items = Vec::new();
                for item in items {
                    let title = item.metadata.get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Movie #{}", item.item_id));

                    email_items.push(json!({
                        "id": item.item_id,
                        "title": title,
                        "reason": item.reasoning.first().cloned().unwrap_or_else(|| "Recommended for you".to_string())
                    }));
                }

                let payload = EmailPayload {
                    profile_id: profile_id.clone(),
                    email: format!("{}@example.com", profile_id), // Mock email resolution
                    subject: "Your personalized picks for the weekend".to_string(),
                    body_html: r#"
                        <h2>Hello!</h2>
                        <p>We've found some new content you might enjoy based on your recent activity:</p>
                        <ul>
                            {{#each items}}
                                <li><strong>{{title}}</strong>: {{reason}}</li>
                            {{/each}}
                        </ul>
                        <p>Open the app to start watching!</p>
                    "#.to_string(),
                    template_slug: "weekly_digest_v1".to_string(),
                    metadata: json!({
                        "items": email_items
                    }),
                };

                let intent = NotificationIntent::Email(payload);

                // 4. Dispatch using the centralized hub
                if let Err(e) = engine.notifications.dispatch(intent).await {
                    warn!(error = %e, profile_id, "Failed to dispatch email digest");
                } else {
                    debug!(profile_id, "Successfully queued email digest");
                }
            }
        }

        Ok(())
    }
}
