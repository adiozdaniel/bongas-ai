//! Experiment and bandit-specific instrumentation.
//!
//! Provides Prometheus metrics for monitoring experimentation infrastructure including
//! multi-armed bandit algorithms, A/B tests, scenario selection, reward tracking,
//! and experiment lifecycle management. Enables statistical analysis and performance
//! monitoring of all experimentation components.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for experimentation and reinforcement learning components.
///
/// Maintains comprehensive metric vectors for:
/// * Experiment execution throughput and reliability
/// * Multi-armed bandit algorithm performance and convergence
/// * A/B test variant assignment and conversion tracking  
/// * Scenario selection dynamics and win rates
/// * Reward processing and distribution analysis
/// * Experiment state transitions and lifecycle events
pub struct ExperimentMetrics {
    /// Total experiment executions, labeled by experiment ID.
    pub experiment_executions: IntCounterVec,
    /// Successful experiment completions, labeled by experiment ID.
    pub experiment_successes: IntCounterVec,
    /// Failed experiment executions, labeled by experiment ID.
    pub experiment_failures: IntCounterVec,
    /// Experiment execution time distribution, labeled by experiment ID.
    pub experiment_execution_latency: HistogramVec,

    /// Bandit algorithm arm selections, labeled by algorithm and arm.
    pub bandit_selections: IntCounterVec,
    /// Cumulative rewards received by bandit algorithms, labeled by algorithm and reward type.
    pub bandit_rewards: IntCounterVec,
    /// Bandit algorithm update events, labeled by algorithm.
    pub bandit_updates: IntCounterVec,
    /// Time to convergence for bandit algorithms, labeled by algorithm.
    pub bandit_convergence: HistogramVec,

    /// A/B test variant assignments, labeled by experiment ID and variant.
    pub ab_test_assignments: IntCounterVec,
    /// A/B test conversion events, labeled by experiment ID and variant.
    pub ab_test_conversions: IntCounterVec,
    /// Conversion rate distribution for A/B test variants.
    pub ab_test_conversion_rate: HistogramVec,

    /// Scenario selections within experiments, labeled by experiment ID and scenario.
    pub scenario_selections: IntCounterVec,
    /// Performance metric distribution for scenarios, labeled by experiment ID and scenario.
    pub scenario_performance: HistogramVec,
    /// Win rate distribution for scenarios, labeled by experiment ID and scenario.
    pub scenario_win_rates: HistogramVec,

    /// Cumulative rewards by experiment and type.
    pub rewards_total: IntCounterVec,
    /// Cumulative rewards aggregated by reward type.
    pub rewards_by_type: IntCounterVec,
    /// Reward processing latency distribution, labeled by reward type.
    pub reward_latency: HistogramVec,

    /// Experiment state transition counts, labeled by experiment ID and state.
    pub experiment_states: IntCounterVec,
    /// Experiment lifecycle event counts, labeled by experiment ID and event.
    pub experiment_lifecycle: IntCounterVec,
}

impl ExperimentMetrics {
    /// Creates and registers all experiment and bandit metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Experiment execution metrics
            experiment_executions: register_int_counter_vec_with_registry!(
                opts!("experiment_executions_total", "Total experiment executions"),
                &[labels::EXPERIMENT_ID],
                registry
            )?,
            experiment_successes: register_int_counter_vec_with_registry!(
                opts!("experiment_successes_total", "Total experiment successes"),
                &[labels::EXPERIMENT_ID],
                registry
            )?,
            experiment_failures: register_int_counter_vec_with_registry!(
                opts!("experiment_failures_total", "Total experiment failures"),
                &[labels::EXPERIMENT_ID],
                registry
            )?,
            experiment_execution_latency: register_histogram_vec_with_registry!(
                format!("{}_experiment_duration_seconds", NAMESPACE),
                "Experiment execution latency",
                &[labels::EXPERIMENT_ID],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Bandit algorithm metrics
            bandit_selections: register_int_counter_vec_with_registry!(
                opts!("bandit_selections_total", "Total bandit algorithm selections"),
                &[labels::BANDIT_ALGORITHM, "arm"],
                registry
            )?,
            bandit_rewards: register_int_counter_vec_with_registry!(
                opts!("bandit_rewards_total", "Total bandit algorithm rewards"),
                &[labels::BANDIT_ALGORITHM, "reward_type"],
                registry
            )?,
            bandit_updates: register_int_counter_vec_with_registry!(
                opts!("bandit_updates_total", "Total bandit algorithm updates"),
                &[labels::BANDIT_ALGORITHM],
                registry
            )?,
            bandit_convergence: register_histogram_vec_with_registry!(
                format!("{}_bandit_convergence_seconds", NAMESPACE),
                "Bandit algorithm convergence time",
                &[labels::BANDIT_ALGORITHM],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // A/B test metrics
            ab_test_assignments: register_int_counter_vec_with_registry!(
                opts!("ab_test_assignments_total", "Total A/B test assignments"),
                &[labels::EXPERIMENT_ID, labels::VARIANT],
                registry
            )?,
            ab_test_conversions: register_int_counter_vec_with_registry!(
                opts!("ab_test_conversions_total", "Total A/B test conversions"),
                &[labels::EXPERIMENT_ID, labels::VARIANT],
                registry
            )?,
            ab_test_conversion_rate: register_histogram_vec_with_registry!(
                format!("{}_ab_test_conversion_rate", NAMESPACE),
                "A/B test conversion rate",
                &[labels::EXPERIMENT_ID, labels::VARIANT],
                vec![0.0, 0.01, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5],
                registry
            )?,

            // Scenario selection metrics
            scenario_selections: register_int_counter_vec_with_registry!(
                opts!("scenario_selections_total", "Total scenario selections"),
                &[labels::EXPERIMENT_ID, labels::SCENARIO],
                registry
            )?,
            scenario_performance: register_histogram_vec_with_registry!(
                format!("{}_scenario_performance", NAMESPACE),
                "Scenario performance metrics",
                &[labels::EXPERIMENT_ID, labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            scenario_win_rates: register_histogram_vec_with_registry!(
                format!("{}_scenario_win_rate", NAMESPACE),
                "Scenario win rates",
                &[labels::EXPERIMENT_ID, labels::SCENARIO],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,

            // Reward tracking metrics
            rewards_total: register_int_counter_vec_with_registry!(
                opts!("rewards_total", "Total rewards received"),
                &[labels::EXPERIMENT_ID, labels::REWARD_TYPE],
                registry
            )?,
            rewards_by_type: register_int_counter_vec_with_registry!(
                opts!("rewards_by_type_total", "Total rewards by type"),
                &[labels::REWARD_TYPE],
                registry
            )?,
            reward_latency: register_histogram_vec_with_registry!(
                format!("{}_reward_latency_seconds", NAMESPACE),
                "Reward processing latency",
                &[labels::REWARD_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Experiment state metrics
            experiment_states: register_int_counter_vec_with_registry!(
                opts!("experiment_states_total", "Total experiment state changes"),
                &[labels::EXPERIMENT_ID, labels::STATE],
                registry
            )?,
            experiment_lifecycle: register_int_counter_vec_with_registry!(
                opts!("experiment_lifecycle_total", "Total experiment lifecycle events"),
                &[labels::EXPERIMENT_ID, labels::LIFECYCLE_EVENT],
                registry
            )?,
        })
    }
}
