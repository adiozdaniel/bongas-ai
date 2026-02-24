//! Circuit breaker registry models.

/// Summary of circuit breaker states in the registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct RegistryStateSummary {
    pub total: usize,
    pub closed: usize,
    pub open: usize,
    pub half_open: usize,
}

impl RegistryStateSummary {
    /// Returns true if all breakers are in Closed state.
    #[inline]
    pub fn all_healthy(&self) -> bool {
        self.open == 0 && self.half_open == 0
    }

    /// Returns the percentage of breakers in Open state.
    #[inline]
    pub fn open_percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.open as f64 / self.total as f64 * 100.0
        }
    }
}
