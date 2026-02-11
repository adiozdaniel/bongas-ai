use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failures exceeded threshold, reject requests
    HalfOpen,    // Testing if service recovered
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,

    // Configuration
    failure_threshold: u64,
    success_threshold: u64,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(
        failure_threshold: u64,
        success_threshold: u64,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_threshold,
            success_threshold,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;

        match *state {
            CircuitState::Open => {
                // Check if timeout expired
                let last_failure = self.last_failure_time.read().await;
                if let Some(last_time) = *last_failure {
                    if last_time.elapsed() > self.timeout {
                        // Transition to HalfOpen
                        drop(state);
                        drop(last_failure);
                        *self.state.write().await = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::Relaxed);
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub async fn record_success(&self) {
        let state = self.state.read().await;

        match *state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                if successes >= self.success_threshold {
                    drop(state);
                    *self.state.write().await = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        if failures >= self.failure_threshold {
            *self.state.write().await = CircuitState::Open;
            *self.last_failure_time.write().await = Some(Instant::now());
        }
    }
}