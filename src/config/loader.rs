//! Configuration loader for the Composite Configuration Pattern.
//!
//! Orchestrates loading configuration from multiple sources with precedence:
//! TOML defaults → ENV overrides → Spring Cloud Config. Provides immutable
//! configuration for Netflix-grade resilience patterns.

use super::sources::{ConfigSource, ConfigResult, ConfigError, TomlSource, EnvSource, SpringCloudSource};
use super::types::{
    AppConfig, CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    IngestionConfig, KafkaConfig, ApiSourceConfig, ClickHouseSourceConfig,
    SecurityConfig, MlConfig, PipelineConfig, ObservabilityConfig, ResilienceConfig,
    HiveMindConfig, SlidingWindowType, BackoffStrategy, ExportFormat,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration loader for the Composite Configuration Pattern.
pub struct ConfigLoader {
    layers: Vec<Box<dyn ConfigSource>>,
}

impl ConfigLoader {
    /// Create a new configuration loader.
    pub fn new() -> Self {
        Self { layers: vec![] }
    }

    /// Add TOML source for default configuration.
    pub fn with_defaults(mut self) -> Self {
        self.layers.push(Box::new(TomlSource::from_file("config/default.toml")));
        self
    }

    /// Add environment variable source for overrides.
    pub fn with_env(mut self) -> Self {
        self.layers.push(Box::new(EnvSource::new()));
        self
    }

    /// Add Spring Cloud Config source for dynamic configuration.
    pub fn with_spring_cloud(mut self, url: String, app_name: String, profile: String) -> Self {
        self.layers.push(Box::new(SpringCloudSource::new(url, app_name, profile)));
        self
    }

    /// Load configuration from all sources and create immutable AppConfig.
    pub fn load(self) -> ConfigResult<AppConfig> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let mut config_map = HashMap::new();

        tracing::info!("Loading application configuration from {} sources...", self.layers.len());

        for source in &self.layers {
            let source_config = source.load()?;
            config_map.extend(source_config);
        }

        let app_config = Self::parse_config_map(config_map)?;
        Self::validate_config(&app_config)?;

        tracing::info!("Application configuration loaded and validated successfully");
        Ok(app_config)
    }

    /// Parse flat configuration map into typed AppConfig.
    fn parse_config_map(config_map: HashMap<String, String>) -> ConfigResult<AppConfig> {
        let parse_val = |key: &str, default: &str| {
            config_map.get(key).cloned().unwrap_or_else(|| default.to_string())
        };

        let parse_u32 = |key: &str, default: u32| -> ConfigResult<u32> {
            let val = parse_val(key, &default.to_string());
            val.parse().map_err(|e| ConfigError::Parse(format!("Invalid u32 for {}: {}", key, e)))
        };

        let parse_u64 = |key: &str, default: u64| -> ConfigResult<u64> {
            let val = parse_val(key, &default.to_string());
            val.parse().map_err(|e| ConfigError::Parse(format!("Invalid u64 for {}: {}", key, e)))
        };

        let parse_bool = |key: &str, default: bool| -> ConfigResult<bool> {
            let val = parse_val(key, &default.to_string());
            val.parse().map_err(|e| ConfigError::Parse(format!("Invalid boolean for {}: {}", key, e)))
        };

        let parse_f64 = |key: &str, default: f64| -> ConfigResult<f64> {
            let val = parse_val(key, &default.to_string());
            val.parse().map_err(|e| ConfigError::Parse(format!("Invalid float for {}: {}", key, e)))
        };

        // Server
        let server = ServerConfig {
            host: parse_val("server.host", "0.0.0.0"),
            port: parse_u32("server.port", 8080)? as u16,
            environment: parse_val("server.environment", "development"),
            tls_enabled: parse_bool("server.tls_enabled", false)?,
            tls_cert_path: config_map.get("server.tls_cert_path").cloned(),
            tls_key_path: config_map.get("server.tls_key_path").cloned(),
            max_connections: parse_u32("server.max_connections", 1000)? as usize,
            request_timeout: parse_u64("server.request_timeout", 30)?,
            keep_alive_timeout: parse_u64("server.keep_alive_timeout", 5)?,
        };

        // Database
        let database = DatabaseConfig {
            url: config_map.get("database.url").cloned(),
            read_replicas: config_map.get("database.read_replicas")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            max_connections: parse_u32("database.max_connections", 20)?,
            min_connections: parse_u32("database.min_connections", 5)?,
            connection_timeout: parse_u64("database.connection_timeout", 30)?,
            idle_timeout: parse_u64("database.idle_timeout", 600)?,
            max_lifetime: parse_u64("database.max_lifetime", 1800)?,
            statement_timeout: parse_u64("database.statement_timeout", 300)?,
            use_read_replicas: parse_bool("database.use_read_replicas", false)?,
        };

        // Redis
        let redis = RedisConfig {
            url: parse_val("redis.url", "redis://localhost:6379"),
            cluster_nodes: config_map.get("redis.cluster_nodes")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            pool_size: parse_u32("redis.pool_size", 10)?,
            connection_timeout: parse_u64("redis.connection_timeout", 5)?,
            request_timeout: parse_u64("redis.request_timeout", 10)?,
            max_retries: parse_u32("redis.max_retries", 3)?,
            retry_backoff: parse_u64("redis.retry_backoff", 100)?,
            cluster_mode: parse_bool("redis.cluster_mode", false)?,
        };

        // ClickHouse
        let clickhouse = ClickHouseConfig {
            url: parse_val("clickhouse.url", "http://localhost:8123"),
            user: parse_val("clickhouse.user", "default"),
            password: parse_val("clickhouse.password", ""),
            database: parse_val("clickhouse.database", "baze_analytics"),
            connection_timeout: parse_u64("clickhouse.connection_timeout", 10)?,
            request_timeout: parse_u64("clickhouse.request_timeout", 60)?,
            max_connections: parse_u32("clickhouse.max_connections", 10)?,
        };

        // Ingestion
        let ingestion = IngestionConfig {
            kafka: KafkaConfig {
                brokers: parse_val("kafka.brokers", "localhost:9092"),
                group_id: parse_val("kafka.group_id", "bongas-ai-consumers"),
                profile_topic: parse_val("kafka.profile_topic", "user.profiles"),
                reaction_topic: parse_val("kafka.reaction_topic", "user.reactions"),
                notification_topic: parse_val("kafka.notification_topic", "notifications"),
                playback_topic: parse_val("kafka.playback_topic", "playback.sessions"),
                connection_timeout: parse_u64("kafka.connection_timeout", 10)?,
                request_timeout: parse_u64("kafka.request_timeout", 30)?,
                max_retries: parse_u32("kafka.max_retries", 3)?,
                retry_backoff: parse_u64("kafka.retry_backoff", 1000)?,
            },
            api: ApiSourceConfig {
                rate_limit_per_second: parse_u64("ingestion.api.rate_limit_per_second", 1000)? as u32,
                batch_size: parse_u32("ingestion.api.batch_size", 100)? as usize,
            },
            clickhouse: ClickHouseSourceConfig {
                poll_interval_secs: parse_u64("ingestion.clickhouse.poll_interval_secs", 60)?,
                batch_size: parse_u32("ingestion.clickhouse.batch_size", 1000)? as usize,
                lookback_window_secs: parse_u64("ingestion.clickhouse.lookback_window_secs", 300)?,
            },
            buffer_size: parse_u32("ingestion.buffer_size", 10_000)? as usize,
            processing_timeout_secs: parse_u64("ingestion.processing_timeout_secs", 30)?,
        };

        // Security
        let mut security = SecurityConfig::default();
        if let Some(v) = config_map.get("security.license_key") { security.license_key = v.clone(); }
        if let Some(v) = config_map.get("security.license_server_url") { security.license_server_url = v.clone(); }
        if let Some(v) = config_map.get("security.mobile_api_key") { security.mobile_api_key = v.clone(); }
        if let Some(v) = config_map.get("security.web_api_key") { security.web_api_key = v.clone(); }
        if let Some(v) = config_map.get("security.tv_api_key") { security.tv_api_key = v.clone(); }
        if let Some(v) = config_map.get("security.system_api_key") { security.system_api_key = v.clone(); }
        if let Some(v) = config_map.get("security.jwt_secret_key") { security.jwt_secret_key = v.clone(); }

        // ML
        let mut ml = MlConfig::default();
        if let Some(v) = config_map.get("ml.model_path") { ml.model_path = PathBuf::from(v); }

        // Pipeline
        let mut pipeline = PipelineConfig::default();
        pipeline.stage_breaker_enabled = parse_bool("pipeline.stage_breaker_enabled", true)?;

        // Experiments
        let experiments = super::types::experiments::ExperimentsConfig {
            enabled: parse_bool("experiments.enabled", false)?,
            assignment_method: parse_val("experiments.assignment_method", "random"),
        };

        // Resilience (Fix #12: Exposed to config)
        let circuit_breaker = CircuitBreakerConfig {
            enabled: parse_bool("resilience.circuit_breaker.enabled", true)?,
            failure_rate_threshold: parse_f64("resilience.circuit_breaker.failure_rate_threshold", 0.5)?,
            slow_call_rate_threshold: parse_f64("resilience.circuit_breaker.slow_call_rate_threshold", 0.5)?,
            slow_call_duration: Duration::from_secs(parse_u64("resilience.circuit_breaker.slow_call_duration_secs", 2)?),
            minimum_calls: parse_u64("resilience.circuit_breaker.minimum_calls", 10)?,
            wait_duration_in_open_state: Duration::from_secs(parse_u64("resilience.circuit_breaker.wait_duration_in_open_state_secs", 30)?),
            permitted_calls_in_half_open_state: parse_u64("resilience.circuit_breaker.permitted_calls_in_half_open_state", 3)?,
            sliding_window_size: parse_u64("resilience.circuit_breaker.sliding_window_size", 100)?,
            sliding_window_type: match parse_val("resilience.circuit_breaker.sliding_window_type", "count").as_str() {
                "time" => SlidingWindowType::TimeBased,
                _ => SlidingWindowType::CountBased,
            },
            writable_stack_trace_enabled: parse_bool("resilience.circuit_breaker.writable_stack_trace_enabled", true)?,
            record_exceptions: vec![],
            ignore_exceptions: vec![],
        };

        let error = ErrorConfig {
            enabled: parse_bool("resilience.error.enabled", true)?,
            retry_enabled: parse_bool("resilience.error.retry_enabled", true)?,
            max_retries: parse_u32("resilience.error.max_retries", 3)?,
            retry_backoff_strategy: match parse_val("resilience.error.retry_backoff_strategy", "exponential").as_str() {
                "linear" => BackoffStrategy::Linear { 
                    increment: Duration::from_millis(100), 
                    max_delay: Duration::from_secs(5) 
                },
                "fixed" => BackoffStrategy::Fixed(Duration::from_millis(100)),
                _ => BackoffStrategy::Exponential { 
                    base: Duration::from_millis(100), 
                    max_delay: Duration::from_secs(5) 
                },
            },
            retry_jitter_enabled: parse_bool("resilience.error.retry_jitter_enabled", true)?,
            retry_timeout: Duration::from_secs(parse_u64("resilience.error.retry_timeout_secs", 30)?),
            error_context_enabled: parse_bool("resilience.error.error_context_enabled", true)?,
            error_context_max_length: parse_u32("resilience.error.error_context_max_length", 1000)? as usize,
            transient_error_patterns: vec![],
            permanent_error_patterns: vec![],
            timeout_error_patterns: vec![],
            overload_error_patterns: vec![],
            degraded_error_patterns: vec![],
            partial_failure_patterns: vec![],
        };

        let analytics = AnalyticsConfig {
            enabled: parse_bool("resilience.analytics.enabled", true)?,
            metrics_collection_interval: Duration::from_secs(parse_u64("resilience.analytics.collection_interval_secs", 10)?),
            histogram_precision: parse_u32("resilience.analytics.histogram_precision", 3)?,
            histogram_max_value: parse_u64("resilience.analytics.histogram_max_value", 60_000_000)?,
            histogram_min_value: parse_u64("resilience.analytics.histogram_min_value", 1)?,
            export_format: match parse_val("resilience.analytics.export_format", "json").as_str() {
                "prometheus" => ExportFormat::Prometheus,
                "csv" => ExportFormat::Csv,
                _ => ExportFormat::Json,
            },
            export_interval: Duration::from_secs(parse_u64("resilience.analytics.export_interval_secs", 60)?),
            export_path: parse_val("resilience.analytics.export_path", "./metrics"),
            error_classification_enabled: parse_bool("resilience.analytics.error_classification_enabled", true)?,
            degraded_failure_tracking_enabled: parse_bool("resilience.analytics.degraded_failure_tracking_enabled", true)?,
            partial_failure_tracking_enabled: parse_bool("resilience.analytics.partial_failure_tracking_enabled", true)?,
            lock_poison_recovery_enabled: parse_bool("resilience.analytics.lock_poison_recovery_enabled", true)?,
            validated_config_enabled: parse_bool("resilience.analytics.validated_config_enabled", true)?,
            max_metrics_history: parse_u32("resilience.analytics.max_metrics_history", 1000)? as usize,
        };

        let resilience = ResilienceConfig {
            defaults: super::types::resilience::ResilienceDefaults {
                circuit_breaker: circuit_breaker.clone(),
                retry: super::types::resilience::RetryConfig {
                    max_retries: error.max_retries,
                    base_delay: match &error.retry_backoff_strategy {
                        BackoffStrategy::Exponential { base, .. } => *base,
                        BackoffStrategy::Fixed(d) => *d,
                        BackoffStrategy::Linear { increment, .. } => *increment,
                    },
                    max_delay: match &error.retry_backoff_strategy {
                        BackoffStrategy::Exponential { max_delay, .. } => *max_delay,
                        BackoffStrategy::Fixed(d) => *d,
                        BackoffStrategy::Linear { max_delay, .. } => *max_delay,
                    },
                },
                timeout: error.retry_timeout,
            },
            overrides: HashMap::new(),
        };

        // Hive Mind
        let hive_mind = HiveMindConfig {
            enabled: parse_bool("hive_mind.enabled", false)?,
            url: parse_val("hive_mind.url", "https://api.bongas-ai/v1/hive-mind"),
            api_key: config_map.get("hive_mind.api_key").cloned(),
            poll_interval_seconds: parse_u64("hive_mind.poll_interval_seconds", 3600)?,
            auto_approve_safe_rules: parse_bool("hive_mind.auto_approve_safe_rules", false)?,
        };

        Ok(AppConfig::new(
            server, database, redis, clickhouse, ingestion, security, ml, pipeline,
            circuit_breaker, error, analytics, ObservabilityConfig::default(),
            resilience, experiments, hive_mind,
        ))
    }

    /// Validate cross-module configuration dependencies.
    fn validate_config(config: &AppConfig) -> ConfigResult<()> {
        super::validation::validate_app_config(config)
            .map_err(|e| ConfigError::Validation(e.to_string()))
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
