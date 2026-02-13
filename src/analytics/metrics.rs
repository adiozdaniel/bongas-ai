//! Shared constants for Prometheus metrics to ensure consistency.
//!
//! Provides centralized definitions for metric namespaces, label keys, and histogram
//! buckets. Using these constants across all metric modules ensures uniform naming
//! conventions and enables consistent querying and aggregation across the entire
//! observability stack.

/// Application-wide metric namespace prefix.
///
/// Prepended to all metric names to avoid naming collisions when multiple
/// applications report to the same Prometheus server.
pub const NAMESPACE: &str = "bongas";

/// Standardized label keys for consistent metric dimensionality.
///
/// All metric collectors should use these constants when defining label names
/// to ensure uniform cardinality and query semantics across components.
pub mod labels {
    /// Kafka topic identifier.
    pub const TOPIC: &str = "topic";
    /// Kafka consumer group identifier.
    pub const GROUP_ID: &str = "group_id";
    /// Database table name.
    pub const TABLE: &str = "table";
    /// Database operation classification (SELECT, INSERT, UPDATE, DELETE).
    pub const QUERY_TYPE: &str = "query_type";
    /// Cache tier or implementation identifier.
    pub const CACHE_TYPE: &str = "cache_type";
    /// Recommendation scenario identifier.
    pub const SCENARIO: &str = "scenario";
    /// Cache invalidation trigger classification.
    pub const INVALIDATION_REASON: &str = "invalidation_reason";
    /// Processing pipeline stage identifier.
    pub const STAGE: &str = "stage";
    /// ML model identifier.
    pub const MODEL_NAME: &str = "model_name";
    /// HTTP middleware component identifier.
    pub const MIDDLEWARE: &str = "middleware";
    /// HTTP response status code.
    pub const STATUS_CODE: &str = "status_code";
    /// Method of HTTP request (GET, POST, etc.).
    pub const METHOD: &str = "method";
    /// Path of HTTP request for route-level metrics.
    pub const PATH: &str = "path";
    /// Error classification for failure analysis.
    pub const ERROR_TYPE: &str = "error_type";
    /// A/B test or experiment identifier.
    pub const EXPERIMENT_ID: &str = "experiment_id";
    /// Multi-armed bandit algorithm identifier.
    pub const BANDIT_ALGORITHM: &str = "bandit_algorithm";
    /// Data repository or persistence layer identifier.
    pub const REPOSITORY: &str = "repository";
    /// Repository operation type.
    pub const OPERATION: &str = "operation";
    /// Security control or check identifier.
    pub const SECURITY_CHECK: &str = "security_check";
    /// License validation status.
    pub const LICENSE_STATUS: &str = "license_status";
    /// Feature type identifier.
    pub const FEATURE_TYPE: &str = "feature_type";
    /// Operation type for resource management
    pub const OPERATION_TYPE: &str = "operation_type";
    /// User type idendifier.
    pub const USER_TYPE: &str = "user_type";
    /// Reward type identifier for bandit algorithms.
    pub const REWARD_TYPE: &str = "reward_type";
    /// Experiment variant identifier for A/B testing.
    pub const VARIANT: &str = "variant";
    /// Experiment lifecycle event type.
    pub const LIFECYCLE_EVENT: &str = "lifecycle_event";
    /// Experiment state identifier for state transition tracking.
    pub const STATE: &str = "state";
    /// Algorithm identifier for multi-armed bandit performance metrics.
    pub const COMPRESSION_ALGORITHM: &str = "algorithm";
    /// Origin of HTTP request for CORS metrics.
    pub const ORIGIN: &str = "origin";
    /// IP address of client for rate limiting metrics.
    pub const IP: &str = "ip";
    /// Pool type for connection pool metrics.
    pub const POOL_TYPE: &str = "pool_type";
    /// Memory type for memory usage metrics.
    pub const MEMORY_TYPE: &str = "memory_type";
    /// Key operation type for key-level metrics.
    pub const KEY_OPERATION_TYPE: &str = "key_operation";
    /// License type for license usage metrics.
    pub const LICENSE_ID: &str = "license_id";
    /// Integrity check type for security metrics.
    pub const INTEGRITY_CHECK_TYPE: &str = "integrity_check_type";
    /// Security heartbeat type for security metrics.
    pub const HEARTBEAT_TYPE: &str = "heartbeat_type";
    /// Security violation type for security metrics.
    pub const VIOLATION_TYPE: &str = "violation_type";
    /// Security block type for security metrics.
    pub const BLOCK_TYPE: &str = "block_type";
    /// Security warning type for security metrics.
    pub const WARNING_TYPE: &str = "warning_type";
    /// Check type for security violation metrics.
    pub const CHECK_TYPE: &str = "check_type";
    /// Hardware type for hardware performance metrics.
    pub const HARDWARE_TYPE: &str = "hardware_type";
    /// Debugger type for debugging metrics.
    pub const DEBUGGER_TYPE: &str = "debugger_type";
    /// Analysis tool type for static analysis metrics.
    pub const ANALYSIS_TOOL_TYPE: &str = "analysis_tool_type";

}

/// Default latency histogram buckets optimized for microservice operations.
///
/// Bucket boundaries (in seconds) are selected to capture the typical latency
/// distribution of request-response services, from sub-millisecond cache hits
/// to multi-second background operations. Distributed exponentially from 1ms to 10s.
pub const DEFAULT_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
