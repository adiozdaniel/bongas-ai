
  //! Observer trait definitions for the Composite Resilience Pattern.
  //!
  //! Observers are decoupled from emitters. The circuit breaker holds an
  //! `Arc<dyn ResilienceObserver>` and notifies it on every event.
  //! Implementations decide what to do: record Prometheus metrics, log via
  //! tracing, push to Kafka, or silently discard.

  use std::sync::Arc;
  use crate::circuit_breaker::observer::event::CircuitBreakerEvent;

  /// Receives events from resilience infrastructure.
  ///
  /// Implementations must be `Send + Sync` because the circuit breaker may
  /// be shared across threads via `Arc`.
  ///
  /// The `on_event` method is intentionally synchronous — observers should
  /// never block. If an observer needs async work (e.g. sending to Kafka),
  /// it should buffer internally and flush on a background task.
  pub trait ResilienceObserver: Send + Sync {
      /// Called by the circuit breaker on every state change or call result.
      fn on_event(&self, event: &CircuitBreakerEvent);
  }

  /// No-op observer that discards all events.
  pub struct NoOpObserver;

  impl ResilienceObserver for NoOpObserver {
      #[inline]
      fn on_event(&self, _event: &CircuitBreakerEvent) {}
  }

  /// Composite observer that fans out events to multiple observers.
  pub struct CompositeObserver {
      observers: Vec<Arc<dyn ResilienceObserver>>,
  }

  impl CompositeObserver {
      pub fn new(observers: Vec<Arc<dyn ResilienceObserver>>) -> Self {
          Self { observers }
      }

      pub fn add(&mut self, observer: Arc<dyn ResilienceObserver>) {
          self.observers.push(observer);
      }

      pub fn len(&self) -> usize {
          self.observers.len()
      }

      pub fn is_empty(&self) -> bool {
          self.observers.is_empty()
      }
  }

  impl ResilienceObserver for CompositeObserver {
      fn on_event(&self, event: &CircuitBreakerEvent) {
          for observer in &self.observers {
              observer.on_event(event);
          }
      }
  }

  impl Default for CompositeObserver {
      fn default() -> Self {
          Self::new(Vec::new())
      }
  }

  /// Logging observer that emits structured tracing events.
  pub struct TracingObserver;

  impl ResilienceObserver for TracingObserver {
      fn on_event(&self, event: &CircuitBreakerEvent) {
          let breaker = event.breaker_id().label();

          match event {
              CircuitBreakerEvent::CallSucceeded { latency, .. } => {
                  tracing::debug!(
                      target: "resilience::circuit_breaker",
                      breaker = %breaker,
                      latency_ms = latency.as_millis() as u64,
                      "call succeeded"
                  );
              }
              CircuitBreakerEvent::CallFailed { latency, classification, .. } => {
                  tracing::warn!(
                      target: "resilience::circuit_breaker",
                      breaker = %breaker,
                      latency_ms = latency.as_millis() as u64,
                      classification = ?classification,
                      "call failed"
                  );
              }

              CircuitBreakerEvent::CallRejected { .. } => { /* ... */ }
              CircuitBreakerEvent::StateChanged { from, to, .. } => { /* ... */ }
              CircuitBreakerEvent::CallTimedOut { timeout, .. } => { /* ... */ }
              CircuitBreakerEvent::SlowCall { latency, threshold, .. } => { /* ... */ }
              CircuitBreakerEvent::MetricsReset { .. } => { /* ... */ }
          }
      }
  }
