//! Central circuit breaker registry implementation.

use std::collections::HashMap;
use std::sync::{Arc, RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};

use super::super::breaker::{CircuitBreaker, CircuitBreakerHealth};
use super::super::config::CircuitBreakerConfig;
use super::super::observer::{CircuitBreakerId, CircuitState, ResilienceObserver, NoOpObserver};
use super::models::RegistryStateSummary;

/// Central registry for circuit breakers.
pub struct CircuitBreakerRegistry {
    breakers: RwLock<HashMap<CircuitBreakerId, Arc<CircuitBreaker>>>,
    default_observer: Arc<dyn ResilienceObserver>,
}

impl CircuitBreakerRegistry {
    /// Create a new registry with no default observer.
    pub fn new() -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
            default_observer: Arc::new(NoOpObserver),
        }
    }

    /// Create a new registry with a default observer for all breakers.
    pub fn with_observer(observer: Arc<dyn ResilienceObserver>) -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
            default_observer: observer,
        }
    }

    /// Get an existing circuit breaker or create a new one with the given config.
    pub fn get_or_create(
        &self,
        id: CircuitBreakerId,
        config: CircuitBreakerConfig,
    ) -> Arc<CircuitBreaker> {
        // Fast path
        {
            let breakers = self.read_breakers();
            if let Some(breaker) = breakers.get(&id) {
                return breaker.clone();
            }
        }

        // Slow path
        let mut breakers = self.write_breakers();

        if let Some(breaker) = breakers.get(&id) {
            return breaker.clone();
        }

        let breaker = Arc::new(CircuitBreaker::new(
            id.clone(),
            config,
            self.default_observer.clone(),
        ));

        breakers.insert(id, breaker.clone());
        breaker
    }

    /// Get an existing circuit breaker or create one with default config.
    pub fn get_or_create_default(&self, id: CircuitBreakerId) -> Arc<CircuitBreaker> {
        self.get_or_create(id, CircuitBreakerConfig::default())
    }

    /// Get an existing circuit breaker by ID.
    pub fn get(&self, id: &CircuitBreakerId) -> Option<Arc<CircuitBreaker>> {
        self.read_breakers().get(id).cloned()
    }

    /// Register an externally created circuit breaker.
    pub fn register(&self, breaker: Arc<CircuitBreaker>) -> Option<Arc<CircuitBreaker>> {
        let id = breaker.id().clone();
        self.write_breakers().insert(id, breaker)
    }

    /// Unregister a circuit breaker by ID.
    pub fn unregister(&self, id: &CircuitBreakerId) -> Option<Arc<CircuitBreaker>> {
        self.write_breakers().remove(id)
    }

    /// List all registered circuit breaker IDs.
    pub fn list_ids(&self) -> Vec<CircuitBreakerId> {
        self.read_breakers().keys().cloned().collect()
    }

    /// Get the number of registered circuit breakers.
    pub fn len(&self) -> usize {
        self.read_breakers().len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.read_breakers().is_empty()
    }

    // ─── Bulk Operations ───────────────────────────────────────────────────

    /// Reset all circuit breakers to Closed state.
    pub fn reset_all(&self) {
        let breakers = self.read_breakers();
        for breaker in breakers.values() {
            breaker.reset();
        }
    }

    /// Reset circuit breakers matching a predicate.
    pub fn reset_where<F>(&self, predicate: F)
    where
        F: Fn(&CircuitBreakerId) -> bool,
    {
        let breakers = self.read_breakers();
        for (id, breaker) in breakers.iter() {
            if predicate(id) {
                breaker.reset();
            }
        }
    }

    // ─── Health & Monitoring ───────────────────────────────────────────────

    /// Get health information for all circuit breakers.
    pub fn health_all(&self) -> Vec<CircuitBreakerHealth> {
        let breakers = self.read_breakers();
        breakers.values().map(|b| b.health()).collect()
    }

    /// Get health information for a specific circuit breaker.
    pub fn health(&self, id: &CircuitBreakerId) -> Option<CircuitBreakerHealth> {
        self.read_breakers().get(id).map(|b| b.health())
    }

    /// Get all circuit breakers currently in Open state.
    pub fn open_breakers(&self) -> Vec<Arc<CircuitBreaker>> {
        let breakers = self.read_breakers();
        breakers
            .values()
            .filter(|b| b.current_state() == CircuitState::Open)
            .cloned()
            .collect()
    }

    /// Get all circuit breakers currently in HalfOpen state.
    pub fn half_open_breakers(&self) -> Vec<Arc<CircuitBreaker>> {
        let breakers = self.read_breakers();
        breakers
            .values()
            .filter(|b| b.current_state() == CircuitState::HalfOpen)
            .cloned()
            .collect()
    }

    /// Get a summary of circuit breaker states.
    pub fn state_summary(&self) -> RegistryStateSummary {
        let breakers = self.read_breakers();
        let mut summary = RegistryStateSummary::default();

        for breaker in breakers.values() {
            match breaker.current_state() {
                CircuitState::Closed => summary.closed += 1,
                CircuitState::Open => summary.open += 1,
                CircuitState::HalfOpen => summary.half_open += 1,
            }
        }

        summary.total = breakers.len();
        summary
    }

    // ─── Internal Lock Helpers ─────────────────────────────────────────────

    #[inline]
    fn read_breakers(&self) -> RwLockReadGuard<'_, HashMap<CircuitBreakerId, Arc<CircuitBreaker>>> {
        self.breakers.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn write_breakers(&self) -> RwLockWriteGuard<'_, HashMap<CircuitBreakerId, Arc<CircuitBreaker>>> {
        self.breakers.write().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
