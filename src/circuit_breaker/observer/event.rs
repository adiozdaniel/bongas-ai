

  //! Event types emitted by resilience infrastructure.

  use std::time::Duration;
  use crate::error::ErrorClassification;

  /// Identifies which circuit breaker emitted the event.
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  pub struct CircuitBreakerId {
      component: &'static str,
      instance: Option<&'static str>,
  }

  impl CircuitBreakerId {
      pub const fn new(component: &'static str) -> Self {
          Self { component, instance: None }
      }

      pub const fn with_instance(component: &'static str, instance: &'static str) -> Self {
          Self { component, instance: Some(instance) }
      }

      pub fn component(&self) -> &'static str {
          self.component
      }

      pub fn instance(&self) -> Option<&'static str> {
          self.instance
      }

      pub fn label(&self) -> String {
          match self.instance {
              Some(inst) => format!("{}_{}", self.component, inst),
              None => self.component.to_string(),
          }
      }
  }

  impl std::fmt::Display for CircuitBreakerId {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self.instance {
              Some(inst) => write!(f, "{}:{}", self.component, inst),
              None => write!(f, "{}", self.component),
          }
      }
  }

  /// Circuit breaker state for event reporting.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum CircuitState {
      Closed,
      Open,
      HalfOpen,
  }

  impl std::fmt::Display for CircuitState {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self {
              CircuitState::Closed => write!(f, "closed"),
              CircuitState::Open => write!(f, "open"),
              CircuitState::HalfOpen => write!(f, "half_open"),
          }
      }
  }

  /// Events emitted by the circuit breaker.
  #[derive(Debug, Clone)]
  pub enum CircuitBreakerEvent {
      CallSucceeded {
          breaker_id: CircuitBreakerId,
          latency: Duration,
      },
      CallFailed {
          breaker_id: CircuitBreakerId,
          latency: Duration,
          classification: ErrorClassification,
      },
      CallRejected {
          breaker_id: CircuitBreakerId,
      },
      StateChanged {
          breaker_id: CircuitBreakerId,
          from: CircuitState,
          to: CircuitState,
      },
      CallTimedOut {
          breaker_id: CircuitBreakerId,
          timeout: Duration,
      },
      SlowCall {
          breaker_id: CircuitBreakerId,
          latency: Duration,
          threshold: Duration,
      },
      MetricsReset {
          breaker_id: CircuitBreakerId,
      },
  }

  impl CircuitBreakerEvent {
      pub fn breaker_id(&self) -> &CircuitBreakerId {
          match self {
              CircuitBreakerEvent::CallSucceeded { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::CallFailed { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::CallRejected { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::StateChanged { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::CallTimedOut { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::SlowCall { breaker_id, .. } => breaker_id,
              CircuitBreakerEvent::MetricsReset { breaker_id, .. } => breaker_id,
          }
      }

      pub fn event_type(&self) -> &'static str {
          match self {
              CircuitBreakerEvent::CallSucceeded { .. } => "call_succeeded",
              CircuitBreakerEvent::CallFailed { .. } => "call_failed",
              CircuitBreakerEvent::CallRejected { .. } => "call_rejected",
              CircuitBreakerEvent::StateChanged { .. } => "state_changed",
              CircuitBreakerEvent::CallTimedOut { .. } => "call_timed_out",
              CircuitBreakerEvent::SlowCall { .. } => "slow_call",
              CircuitBreakerEvent::MetricsReset { .. } => "metrics_reset",
          }
      }

      /// Returns true if this event indicates a problem
      pub fn is_problem(&self) -> bool {
          matches!(
              self,
              CircuitBreakerEvent::CallFailed { .. }
                  | CircuitBreakerEvent::CallRejected { .. }
                  | CircuitBreakerEvent::CallTimedOut { .. }
                  | CircuitBreakerEvent::SlowCall { .. }
          )
      }

      /// Returns the latency if this event type has one.
      pub fn latency(&self) -> Option<Duration> {
          match self {
              CircuitBreakerEvent::CallSucceeded { latency, .. } => Some(*latency),
              CircuitBreakerEvent::CallFailed { latency, .. } => Some(*latency),
              CircuitBreakerEvent::CallTimedOut { timeout, .. } => Some(*timeout),
              CircuitBreakerEvent::SlowCall { latency, .. } => Some(*latency),
              _ => None,
          }
      }
  }
