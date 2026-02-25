use std::time::Duration;

/// Backoff strategy for retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BackoffStrategy {
    /// No delay between retries.
    None,
    /// Fixed delay between retries.
    #[default]
    Fixed,
    /// Exponential backoff with optional jitter.
    Exponential,
    /// Linear increase in delay.
    Linear,
}

/// Hints for retry behavior, provided by the error source.
///
/// Resilience infrastructure uses these hints to make intelligent retry decisions
/// without hardcoding domain-specific logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryHint {
    /// Suggested backoff strategy.
    pub strategy: BackoffStrategy,
    /// Base delay for backoff calculation.
    pub base_delay: Duration,
    /// Maximum number of retries (None = use default).
    pub max_retries: Option<u32>,
    /// Absolute time after which retry is allowed (e.g., from Retry-After header).
    pub retry_after: Option<Duration>,
    /// Whether to add jitter to prevent thundering herd.
    pub add_jitter: bool,
}

impl Default for RetryHint {
    fn default() -> Self {
        Self {
            strategy: BackoffStrategy::Exponential,
            base_delay: Duration::from_millis(100),
            max_retries: Some(3),
            retry_after: None,
            add_jitter: true,
        }
    }
}

impl RetryHint {
    /// Create a hint for immediate retry (transient errors).
    pub fn immediate() -> Self {
        Self {
            strategy: BackoffStrategy::None,
            base_delay: Duration::ZERO,
            max_retries: Some(3),
            retry_after: None,
            add_jitter: false,
        }
    }

    /// Create a hint for exponential backoff (default).
    pub fn exponential(base_delay: Duration) -> Self {
        Self {
            strategy: BackoffStrategy::Exponential,
            base_delay,
            max_retries: Some(3),
            retry_after: None,
            add_jitter: true,
        }
    }

    /// Create a hint with a specific retry-after duration (e.g., from 429 response).
    pub fn after(duration: Duration) -> Self {
        Self {
            strategy: BackoffStrategy::Fixed,
            base_delay: duration,
            max_retries: Some(1),
            retry_after: Some(duration),
            add_jitter: false,
        }
    }

    /// Create a hint indicating no retry should be attempted.
    pub fn no_retry() -> Self {
        Self {
            strategy: BackoffStrategy::None,
            base_delay: Duration::ZERO,
            max_retries: Some(0),
            retry_after: None,
            add_jitter: false,
        }
    }

    /// Set maximum retries.
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = Some(max);
        self
      }

    /// Enable or disable jitter.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.add_jitter = jitter;
        self
    }
}
