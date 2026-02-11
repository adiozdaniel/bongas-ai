use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed = 0,
    /// Circuit is open, requests are rejected
    Open = 1,
    /// Circuit is testing if the service has recovered
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(v: u8) -> Self {
        match v {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u64,
    /// Number of successes in half-open state before closing
    pub success_threshold: u64,
    /// Duration to wait before transitioning from open to half-open
    pub reset_timeout: Duration,
    /// Duration window to count failures
    pub failure_window: Duration,
    /// Name for logging/metrics
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            failure_window: Duration::from_secs(60),
            name: "default".to_string(),
        }
    }
}

/// Circuit breaker implementation for Kafka consumers
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: AtomicU8,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    last_state_change: RwLock<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            last_failure_time: RwLock::new(None),
            last_state_change: RwLock::new(Instant::now()),
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        CircuitState::from(self.state.load(Ordering::SeqCst))
    }

    /// Check if request should be allowed
    pub async fn allow_request(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let last_change = *self.last_state_change.read().await;
                if last_change.elapsed() >= self.config.reset_timeout {
                    self.transition_to(CircuitState::HalfOpen).await;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    self.transition_to(CircuitState::Closed).await;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle it
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let state = self.state();
        let now = Instant::now();

        // Update last failure time
        *self.last_failure_time.write().await = Some(now);

        match state {
            CircuitState::Closed => {
                // Check if failures are within the window
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

                if count >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open).await;
                }
            }
            CircuitState::HalfOpen => {
                // Single failure in half-open returns to open
                self.transition_to(CircuitState::Open).await;
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Transition to a new state
    async fn transition_to(&self, new_state: CircuitState) {
        let old_state = CircuitState::from(self.state.swap(new_state as u8, Ordering::SeqCst));

        if old_state != new_state {
            *self.last_state_change.write().await = Instant::now();

            // Reset counters on state change
            match new_state {
                CircuitState::Closed => {
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!(
                        circuit = %self.config.name,
                        "Circuit breaker closed - service recovered"
                    );
                }
                CircuitState::Open => {
                    self.success_count.store(0, Ordering::SeqCst);
                    warn!(
                        circuit = %self.config.name,
                        failures = self.failure_count.load(Ordering::SeqCst),
                        "Circuit breaker opened - too many failures"
                    );
                }
                CircuitState::HalfOpen => {
                    self.success_count.store(0, Ordering::SeqCst);
                    info!(
                        circuit = %self.config.name,
                        "Circuit breaker half-open - testing recovery"
                    );
                }
            }
        }
    }

    /// Get circuit breaker stats
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.config.name.clone(),
            state: self.state(),
            failure_count: self.failure_count.load(Ordering::SeqCst),
            success_count: self.success_count.load(Ordering::SeqCst),
            failure_threshold: self.config.failure_threshold,
            success_threshold: self.config.success_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub failure_threshold: u64,
    pub success_threshold: u64,
}

impl CircuitBreakerStats {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "state": format!("{:?}", self.state),
            "failure_count": self.failure_count,
            "success_count": self.success_count,
            "failure_threshold": self.failure_threshold,
            "success_threshold": self.success_threshold,
        })
    }
}

/// Helper trait for executing operations with circuit breaker protection
#[async_trait::async_trait]
pub trait CircuitBreakerExecutor {
    type Output;
    type Error;

    async fn execute_with_circuit_breaker(
        &self,
        circuit_breaker: &CircuitBreaker,
    ) -> Result<Self::Output, CircuitBreakerError<Self::Error>>;
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request was rejected
    CircuitOpen,
    /// Operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitBreakerError<E> {}
