//! Alert observer for circuit breaker state change notifications.
//!
//! Sends alerts to external systems (Slack, PagerDuty, webhooks) when
//! circuit breakers change state or error thresholds are exceeded.

use std::sync::Arc;
use std::time::Duration;

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

    let mut fields_json = String::new();
    for (key, value) in &self.fields {
      if !fields_json.is_empty() {
          fields_json.push_str(",");
      }
              
      fields_json.push_str(&format!(
        r#"{{"title":"{}","value":"{}","short":true}}"#,
        key, value
      ));
    }

    format!(
      r#"{{"attachments":[{{"color":"{}","title":"[{}] {}","text":"{}","fields":[{}],"footer":"Circuit Breaker: 
      {}","ts":{}}}]}}"#,
      color,
      self.severity,
      self.title,
      self.description,
      fields_json,
      self.breaker_id,
      self.timestamp
          .duration_since(std::time::UNIX_EPOCH)
          .map(|d| d.as_secs())
          .unwrap_or(0)
    )
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
}

impl AlertObserver {
  /// Create a new alert observer with the given configuration.
  pub fn new(config: AlertConfig) -> Self {
    Self {
      config,
      http_client: None,
    }
  }

  /// Create with a custom HTTP client for sending webhooks.
  pub fn with_sender(config: AlertConfig, sender: Arc<dyn AlertSender>) -> Self {
    Self {
      config,
      http_client: Some(sender),
    }
  }

  /// Send an alert through all configured channels.
  fn send_alert(&self, message: AlertMessage) {
    if message.severity < self.config.min_severity {
      return;
    }

    for channel in &self.config.channels {
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
          if let Some(ref client) = self.http_client {
            let json = message.to_slack_json();
            let url = webhook_url.clone();
            let client = Arc::clone(client);
              
            // Fire and forget - don't block the observer
            tokio::spawn(async move {
              if let Err(e) = client.send(&url, &json).await {
                tracing::error!(error = %e, "failed to send Slack alert");
              }
            });
          }
        }
          
        AlertChannel::Webhook { url, auth_header: _ } => {
          if let Some(ref client) = self.http_client {
            let json = message.to_slack_json(); // Reuse format for now
            let url = url.clone();
            let client = Arc::clone(client);

            tokio::spawn(async move {
              if let Err(e) = client.send(&url, &json).await {
                tracing::error!(error = %e, "failed to send webhook alert");
              }
            });
          }
        }
        
        AlertChannel::None => {}
      }
    }
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
