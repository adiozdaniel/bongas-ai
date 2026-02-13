//! PostgreSQL-specific instrumentation for both cache and database operations.
//!
//! Provides comprehensive Prometheus metrics for monitoring PostgreSQL database
//! interactions including repository operations, query execution, connection pooling,
//! and transaction management. Enables observability of persistence layer performance
//! and reliability.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for PostgreSQL database operations.
///
/// Maintains comprehensive metric vectors for:
/// * Repository CRUD operation throughput and success rates
/// * Query execution volume and latency by table and operation type
/// * Connection pool acquisition and release patterns
/// * Transaction lifecycle and commit/rollback ratios
/// * Operation latency distributions for performance monitoring
pub struct PostgresMetrics {
    /// Total repository operations, labeled by repository and operation type.
    pub repository_operations: IntCounterVec,
    /// Successful repository operations, labeled by repository and operation type.
    pub repository_successes: IntCounterVec,
    /// Failed repository operations, labeled by repository and operation type.
    pub repository_failures: IntCounterVec,
    /// Repository operation latency distribution, labeled by repository and operation.
    pub repository_operation_latency: HistogramVec,

    /// Total query executions, labeled by table and query type.
    pub query_executions: IntCounterVec,
    /// Successful query executions, labeled by table and query type.
    pub query_successes: IntCounterVec,
    /// Failed query executions, labeled by table and query type.
    pub query_failures: IntCounterVec,
    /// Query execution latency distribution, labeled by table and query type.
    pub query_execution_latency: HistogramVec,

    /// Total connection acquisitions from pool, labeled by repository.
    pub connection_acquisitions: IntCounterVec,
    /// Total connection releases back to pool, labeled by repository.
    pub connection_releases: IntCounterVec,
    /// Connection wait time distribution, labeled by repository.
    pub connection_wait_time: HistogramVec,
    /// Total transactions started, labeled by repository.
    pub transactions_started: IntCounterVec,
    /// Total transactions committed, labeled by repository.
    pub transactions_committed: IntCounterVec,
    /// Total transactions rolled back, labeled by repository.
    pub transactions_rolled_back: IntCounterVec,
    /// Transaction duration distribution, labeled by repository.
    pub transaction_latency: HistogramVec,
}

impl PostgresMetrics {
    /// Creates and registers all PostgreSQL metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Regular database operations
            repository_operations: register_int_counter_vec_with_registry!(
                opts!("postgres_repository_operations_total", "Total PostgreSQL repository operations"),
                &[labels::REPOSITORY, labels::OPERATION],
                registry
            )?,
            repository_successes: register_int_counter_vec_with_registry!(
                opts!("postgres_repository_successes_total", "Total PostgreSQL repository operation successes"),
                &[labels::REPOSITORY, labels::OPERATION],
                registry
            )?,
            repository_failures: register_int_counter_vec_with_registry!(
                opts!("postgres_repository_failures_total", "Total PostgreSQL repository operation failures"),
                &[labels::REPOSITORY, labels::OPERATION],
                registry
            )?,
            repository_operation_latency: register_histogram_vec_with_registry!(
                format!("{}_postgres_repository_duration_seconds", NAMESPACE),
                "PostgreSQL repository operation latency",
                &[labels::REPOSITORY, labels::OPERATION],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Query operations
            query_executions: register_int_counter_vec_with_registry!(
                opts!("postgres_query_executions_total", "Total PostgreSQL query executions"),
                &[labels::TABLE, labels::QUERY_TYPE],
                registry
            )?,
            query_successes: register_int_counter_vec_with_registry!(
                opts!("postgres_query_successes_total", "Total PostgreSQL query successes"),
                &[labels::TABLE, labels::QUERY_TYPE],
                registry
            )?,
            query_failures: register_int_counter_vec_with_registry!(
                opts!("postgres_query_failures_total", "Total PostgreSQL query failures"),
                &[labels::TABLE, labels::QUERY_TYPE],
                registry
            )?,
            query_execution_latency: register_histogram_vec_with_registry!(
                format!("{}_postgres_query_duration_seconds", NAMESPACE),
                "PostgreSQL query execution latency",
                &[labels::TABLE, labels::QUERY_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Connection and transaction management
            connection_acquisitions: register_int_counter_vec_with_registry!(
                opts!("postgres_connection_acquisitions_total", "Total PostgreSQL connection acquisitions"),
                &[labels::REPOSITORY],
                registry
            )?,
            connection_releases: register_int_counter_vec_with_registry!(
                opts!("postgres_connection_releases_total", "Total PostgreSQL connection releases"),
                &[labels::REPOSITORY],
                registry
            )?,
            connection_wait_time: register_histogram_vec_with_registry!(
                format!("{}_postgres_connection_wait_seconds", NAMESPACE),
                "PostgreSQL connection wait time",
                &[labels::REPOSITORY],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            transactions_started: register_int_counter_vec_with_registry!(
                opts!("postgres_transactions_started_total", "Total PostgreSQL transactions started"),
                &[labels::REPOSITORY],
                registry
            )?,
            transactions_committed: register_int_counter_vec_with_registry!(
                opts!("postgres_transactions_committed_total", "Total PostgreSQL transactions committed"),
                &[labels::REPOSITORY],
                registry
            )?,
            transactions_rolled_back: register_int_counter_vec_with_registry!(
                opts!("postgres_transactions_rolled_back_total", "Total PostgreSQL transactions rolled back"),
                &[labels::REPOSITORY],
                registry
            )?,
            transaction_latency: register_histogram_vec_with_registry!(
                format!("{}_postgres_transaction_duration_seconds", NAMESPACE),
                "PostgreSQL transaction latency",
                &[labels::REPOSITORY],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
