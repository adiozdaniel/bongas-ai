//! Alert observer for circuit breaker state change notifications.
//!
//! Sends alerts to external systems (Slack, PagerDuty, webhooks) when
//! circuit breakers change state or error thresholds are exceeded.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::Mutex;

use super::event::{CircuitBreakerEvent, CircuitState};
use super::traits::ResilienceObserver;

// ─── Alert Channel ──────────────────────────────────────────────────────────

/// Target for alert notifications.
#[derive(Debug, Clone)]
pub enum AlertChannel {
  /// Slack webhook.
      Slack { webhook_url: String },
  /// Generic webhook.
      Webhook { url: String, auth_header: Option<String> },
  /// Log only (for testing).
      Log,
  /// No-op (disabled).
      None,
  }

// ─── Alert Severity ─────────────────────────────────────────────────────────

/// Severity level for alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
  Info,
  Warning,
  Critical,
}

impl std::fmt::Display for AlertSeverity {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AlertSeverity::Info => write!(f, "INFO"),
      AlertSeverity::Warning => write!(f, "WARNING"),
      AlertSeverity::Critical => write!(f, "CRITICAL"),
    }
  }
}

// ─── Alert Message ──────────────────────────────────────────────────────────

/// Alert message payload.
#[derive(Debug, Clone)]
pub struct AlertMessage {
  pub severity: AlertSeverity,
  pub title: String,
  pub description: String,
  pub breaker_id: String,
  pub timestamp: std::time::SystemTime,
  pub fields: Vec<(String, String)>,
}

impl AlertMessage {
/// Create a new alert message.
  pub fn new(severity: AlertSeverity, breaker_id: impl Into<String>, title: impl Into<String>) -> Self {
    Self {
      severity,
      title: title.into(),
      description: String::new(),
      breaker_id: breaker_id.into(),
      timestamp: std::time::SystemTime::now(),
      fields: Vec::new(),
    }
  }

  /// Set the description.
  pub fn with_description(mut self, description: impl Into<String>) -> Self {
    self.description = description.into();
    self
  }

  /// Add a field.
  pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.fields.push((key.into(), value.into()));
    self
  }

  /// Format as Slack message JSON.
  pub fn to_slack_json(&self) -> String {
    let color = match self.severity {
      AlertSeverity::Info => "#36a64f",
      AlertSeverity::Warning => "#ffcc00",
      AlertSeverity::Critical => "#ff0000",
    };

    let fields: Vec<serde_json::Value> = self.fields.iter().map(|(k, v)| {
      serde_json::json!({
        "title": k,
        "value": v,
        "short": true
      })
    }).collect();

    serde_json::json!({
      "attachments": [{
        "color": color,
        "title": format!("[{}] {}", self.severity, self.title),
        "text": self.description,
        "fields": fields,
        "footer": format!("BONGAS-AI | {}", self.breaker_id),
        "ts": self.timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
      }]
    }).to_string()
  }

  /// Format as generic webhook JSON.
  pub fn to_webhook_json(&self) -> String {
    let fields_map: HashMap<String, String> = self.fields.iter().cloned().collect();
    serde_json::json!({
      "severity": self.severity.to_string(),
      "breaker_id": self.breaker_id,
      "title": self.title,
      "description": self.description,
      "timestamp": self.timestamp,
      "fields": fields_map
    }).to_string()
  }
}

// ─── Alert Configuration ────────────────────────────────────────────────────

/// Configuration for alert observer.
#[derive(Debug, Clone)]
  pub struct AlertConfig {
  /// Alert channels to notify.
  pub channels: Vec<AlertChannel>,
  /// Minimum severity to send alerts.
  pub min_severity: AlertSeverity,
  /// Whether to alert on circuit open.
  pub alert_on_open: bool,
  /// Whether to alert on circuit close.
  pub alert_on_close: bool,
  /// Whether to alert on circuit half-open.
  pub alert_on_half_open: bool,
  /// Cooldown between alerts for the same breaker.
  pub cooldown: Duration,
}

impl Default for AlertConfig {
  fn default() -> Self {
    Self {
      channels: vec![AlertChannel::Log],
      min_severity: AlertSeverity::Warning,
      alert_on_open: true,
      alert_on_close: true,
      alert_on_half_open: false,
      cooldown: Duration::from_secs(60),
    }
  }
}

// ─── Alert Observer ─────────────────────────────────────────────────────────

/// Observer that sends alerts on circuit breaker state changes.
pub struct AlertObserver {
    config: AlertConfig,
    http_client: Option<Arc<dyn AlertSender>>,
    last_alert_times: Arc<Mutex<HashMap<String, Instant>>>,
}

impl AlertObserver {
  /// Create a new alert observer with the given configuration.
  pub fn new(config: AlertConfig) -> Self {
    Self {
      config,
      http_client: None,
      last_alert_times: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  /// Create with a custom HTTP client for sending webhooks.
  pub fn with_sender(config: AlertConfig, sender: Arc<dyn AlertSender>) -> Self {
    Self {
      config,
      http_client: Some(sender),
      last_alert_times: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  /// Send an alert through all configured channels.
  fn send_alert(&self, message: AlertMessage) {
    if message.severity < self.config.min_severity {
      return;
    }

    let config = self.config.clone();
    let http_client = self.http_client.clone();
    let last_alert_times = self.last_alert_times.clone();
    let breaker_id = message.breaker_id.clone();

    // Perform the actual check and sending in a background task 
    // because ResilienceObserver::on_event is synchronous.
    tokio::spawn(async move {
        // Enforce cooldown
        {
          let mut times = last_alert_times.lock().await;
          if let Some(last) = times.get(&breaker_id) {
            if last.elapsed() < config.cooldown {
              return; // Still in cooldown
            }
          }
          times.insert(breaker_id, Instant::now());
        }

        for channel in &config.channels {
          match channel {
            AlertChannel::Log => {
              tracing::warn!(
                severity = %message.severity,
                breaker_id = %message.breaker_id,
                title = %message.title,
                description = %message.description,
                "circuit breaker alert"
              );
            }
              
            AlertChannel::Slack { webhook_url } => {
              if let Some(ref client) = http_client {
                let json = message.to_slack_json();
                let url = webhook_url.clone();
                  
                if let Err(e) = client.send(&url, &json).await {
                    tracing::error!(error = %e, "failed to send Slack alert");
                }
              }
            }
              
            AlertChannel::Webhook { url, auth_header } => {
              if let Some(ref client) = http_client {
                let json = message.to_webhook_json();
                let url = url.clone();
                let auth = auth_header.clone();

                if let Err(e) = client.send_with_auth(&url, &json, auth.as_deref()).await {
                    tracing::error!(error = %e, "failed to send webhook alert");
                }
              }
            }
            
            AlertChannel::None => {}
          }
        }
    });
  }

  /// Determine severity based on state transition.
  fn severity_for_transition(&self, _from: CircuitState, to: CircuitState) -> AlertSeverity {
    match to {
      CircuitState::Open => AlertSeverity::Critical,
      CircuitState::HalfOpen => AlertSeverity::Warning,
      CircuitState::Closed => AlertSeverity::Info,
    }
  }
}

impl ResilienceObserver for AlertObserver {
  fn on_event(&self, event: &CircuitBreakerEvent) {
    match event {
      CircuitBreakerEvent::StateChanged { breaker_id, from, to } => {
        let should_alert = match to {
          CircuitState::Open => self.config.alert_on_open,
          CircuitState::Closed => self.config.alert_on_close,
          CircuitState::HalfOpen => self.config.alert_on_half_open,
        };

        if !should_alert {
          return;
        }

        let severity = self.severity_for_transition(*from, *to);
        let title = format!("Circuit breaker {} → {}", from, to);
        let description = format!(
          "Circuit breaker '{}' transitioned from {} to {}",
          breaker_id.label(),
          from,
          to
        );

        let message = AlertMessage::new(severity, breaker_id.label(), title)
          .with_description(description)
          .with_field("From State", from.to_string())
          .with_field("To State", to.to_string())
          .with_field("Component", breaker_id.component());

        self.send_alert(message);
      }
      
      _ => {
          // Only alert on state changes for now
      }
    }
  }
}

// ─── Alert Sender Trait ─────────────────────────────────────────────────────

/// Trait for sending HTTP alerts (allows mocking in tests).
#[async_trait::async_trait]
pub trait AlertSender: Send + Sync {
  /// Send an alert payload to the given URL.
  async fn send(&self, url: &str, payload: &str) -> Result<(), AlertSendError>;

  /// Send an alert payload with optional auth header.
  async fn send_with_auth(
    &self, url: &str, payload: &str, _auth_header: Option<&str>
  ) -> Result<(), AlertSendError> {
    self.send(url, payload).await
  }
}

/// Error sending alert.
#[derive(Debug)]
pub struct AlertSendError {
  pub message: String,
}

impl std::fmt::Display for AlertSendError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "failed to send alert: {}", self.message)
  }
}

impl std::error::Error for AlertSendError {}
