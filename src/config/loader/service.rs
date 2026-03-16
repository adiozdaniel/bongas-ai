use crate::config::sources::{ConfigSource, ConfigResult, ConfigError, TomlSource, EnvSource, SpringCloudSource};
use crate::config::types::{
    AppConfig, CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    IngestionConfig, KafkaConfig, ApiSourceConfig, ClickHouseSourceConfig,
    SecurityConfig, MlConfig, ExposureSourceAdaptor, PipelineConfig, ObservabilityConfig, ResilienceConfig,
    NotificationConfig, ResendConfig, NotificationAdaptorKind,
    HiveMindConfig, SearchConfig, SlidingWindowType, BackoffStrategy, ExportFormat,
    experiments::ExperimentsConfig, resilience::{ResilienceDefaults, RetryConfig},
};
use crate::cache::CacheConfig;
use crate::experiments::models::AssignmentMethod;
use crate::resilience::ResilienceMetricsConfig;
use crate::config::validation::validate_app_config as validate_config_fn;
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
        Self::validate_app_config(&app_config)?;

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
            max_connections: parse_u32("database.max_connections", 100)?,
            min_connections: parse_u32("database.min_connections", 20)?,
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
            pool_size: parse_u32("redis.pool_size", 50)?,
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
                enabled: parse_bool("kafka.enabled", false)?,
                brokers: parse_val("kafka.brokers", "localhost:9092"),
                group_id: parse_val("kafka.group_id", "bongas-ai-consumers"),
                profile_topic: parse_val("kafka.profile_topic", "profile.events"),
                reaction_topic: parse_val("kafka.reaction_topic", "reaction.events"),
                notification_topic: parse_val("kafka.notification_topic", "notification.events"),
                playback_topic: parse_val("kafka.playback_topic", "playback.events"),
                sync_topic: parse_val("kafka.sync_topic", "recommendations.sync"),
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

        // ML
        let ml = MlConfig {
            model_path: PathBuf::from(parse_val("ml.model_path", "./models")),
            batch_size: parse_u32("ml.batch_size", 64)? as usize,
            onnx_enabled: parse_bool("onnx.enabled", true)?,
            onnx_execution_provider: parse_val("onnx.execution_provider", "cpu"),
            onnx_graph_optimization: parse_bool("onnx.graph_optimization", true)?,
            onnx_memory_map: parse_bool("onnx.memory_map", true)?,
            onnx_intra_threads: parse_u32("onnx.intra_threads", 4)? as usize,
            feature_store_enabled: parse_bool("ml.feature_store_enabled", true)?,
            feature_cache_ttl: Duration::from_secs(parse_u64("ml.feature_cache_ttl_secs", 300)?),
            feature_fetch_timeout: Duration::from_millis(parse_u64("ml.feature_fetch_timeout_ms", 500)?),
            model_cache_size: parse_u32("ml.model_cache_size", 100)? as usize,
            canary_enabled: parse_bool("ml.canary_enabled", false)?,
            canary_traffic_percent: parse_f64("ml.canary_traffic_percent", 5.0)?,
            shadow_mode_enabled: parse_bool("ml.shadow_mode_enabled", false)?,
            online_learning_enabled: parse_bool("ml.online_learning_enabled", false)?,
            feedback_batch_size: parse_u32("ml.feedback_batch_size", 256)? as usize,
            feedback_flush_interval: Duration::from_secs(parse_u64("ml.feedback_flush_interval_secs", 30)?),
            inference_breaker_failure_rate: parse_f64("ml.inference_breaker_failure_rate", 0.5)?,
            inference_breaker_slow_call_rate: parse_f64("ml.inference_breaker_slow_call_rate", 0.5)?,
            inference_breaker_slow_call_duration: Duration::from_secs(parse_u64("ml.inference_breaker_slow_call_duration_secs", 2)?),
            inference_breaker_minimum_calls: parse_u64("ml.inference_breaker_minimum_calls", 10)?,
            inference_breaker_recovery_timeout: Duration::from_secs(parse_u64("ml.inference_breaker_recovery_timeout_secs", 30)?),
            inference_breaker_half_open_calls: parse_u32("ml.inference_breaker_half_open_calls", 3)? as usize,
            inference_max_concurrent: parse_u32("ml.inference_max_concurrent", 16)? as usize,
            feature_fetch_max_concurrent: parse_u32("ml.feature_fetch_max_concurrent", 32)? as usize,
            worker_queue_depth: parse_u32("ml.worker_queue_depth", 1024)? as usize,
            model_load_max_retries: parse_u32("ml.model_load_max_retries", 3)? as usize,
            model_load_base_backoff: Duration::from_millis(parse_u64("ml.model_load_base_backoff_ms", 100)?),
            model_load_max_backoff: Duration::from_secs(parse_u64("ml.model_load_max_backoff_secs", 5)?),
            feature_fetch_max_retries: parse_u32("ml.feature_fetch_max_retries", 2)? as usize,
            inference_timeout: Duration::from_secs(parse_u64("ml.inference_timeout_secs", 5)?),
            model_load_timeout: Duration::from_secs(parse_u64("ml.model_load_timeout_secs", 30)?),
            fallback_to_stale_model: parse_bool("ml.fallback_to_stale_model", true)?,
            fallback_cold_start_score: parse_f64("ml.fallback_cold_start_score", 0.5)? as f32,
            fallback_max_stale_age: Duration::from_secs(parse_u64("ml.fallback_max_stale_age_secs", 3600)?),
            analytics_enabled: parse_bool("ml.analytics_enabled", true)?,
            analytics_sample_rate: parse_f64("ml.analytics_sample_rate", 1.0)?,
            central_server_url: parse_val("ml.central_server_url", "https://ml.bongas-ai.com"),
            tribe_num_clusters: parse_u32("ml.tribe_num_clusters", 100)? as usize,
            tribe_clustering_interval: Duration::from_secs(parse_u64("ml.tribe_clustering_interval_secs", 14400)?),
            fatigue_enabled: parse_bool("ml.fatigue_enabled", true)?,
            fatigue_adaptor: match parse_val("ml.fatigue_adaptor", "internal_hook").as_str() {
                "kafka_stream" => ExposureSourceAdaptor::KafkaStream,
                "clickhouse_poll" => ExposureSourceAdaptor::ClickHousePoll,
                _ => ExposureSourceAdaptor::InternalHook,
            },
            fatigue_max_exposures: parse_u32("ml.fatigue_max_exposures", 5)?,
            fatigue_penalty_factor: parse_f64("ml.fatigue_penalty_factor", 0.8)? as f32,
        };

        // Pipeline
        let pipeline = PipelineConfig {
            stage_breaker_enabled: parse_bool("pipeline.stage_breaker_enabled", true)?,
            stage_breaker_failure_rate: parse_f64("pipeline.stage_breaker_failure_rate", 0.5)?,
            stage_breaker_slow_call_rate: parse_f64("pipeline.stage_breaker_slow_call_rate", 0.5)?,
            stage_breaker_slow_call_duration: Duration::from_secs(parse_u64("pipeline.stage_breaker_slow_call_duration_secs", 2)?),
            stage_breaker_minimum_calls: parse_u64("pipeline.stage_breaker_minimum_calls", 10)?,
            stage_breaker_recovery_timeout: Duration::from_secs(parse_u64("pipeline.stage_breaker_recovery_timeout_secs", 30)?),
            stage_breaker_half_open_calls: parse_u32("pipeline.stage_breaker_half_open_calls", 3)? as usize,
            stage_timeout_default: Duration::from_secs(parse_u64("pipeline.stage_timeout_default_secs", 5)?),
            fetch_stage_timeout: Duration::from_secs(parse_u64("pipeline.fetch_stage_timeout_secs", 3)?),
            ml_stage_timeout: Duration::from_secs(parse_u64("pipeline.ml_stage_timeout_secs", 10)?),
            filter_stage_timeout: Duration::from_secs(parse_u64("pipeline.filter_stage_timeout_secs", 2)?),
            pipeline_timeout: Duration::from_secs(parse_u64("pipeline.pipeline_timeout_secs", 30)?),
            stage_max_concurrent: parse_u32("pipeline.stage_max_concurrent", 32)? as usize,
            fetch_max_concurrent: parse_u32("pipeline.fetch_max_concurrent", 16)? as usize,
            ml_max_concurrent: parse_u32("pipeline.ml_max_concurrent", 8)? as usize,
            fallback_enabled: parse_bool("pipeline.fallback_enabled", true)?,
            fallback_on_stage_timeout: parse_bool("pipeline.fallback_on_stage_timeout", true)?,
            fallback_on_stage_error: parse_bool("pipeline.fallback_on_stage_error", true)?,
            fallback_pass_through_input: parse_bool("pipeline.fallback_pass_through_input", true)?,
            analytics_enabled: parse_bool("pipeline.analytics_enabled", true)?,
            analytics_per_stage: parse_bool("pipeline.analytics_per_stage", true)?,
            analytics_sample_rate: parse_f64("pipeline.analytics_sample_rate", 1.0)?,
        };

        // Cache
        let cache = CacheConfig {
            l1_enabled: true,
            l1_max_entries: 10000,
            l1_ttl: Duration::from_secs(parse_u64("cache.l1_ttl_seconds", 300)?),
            l2_enabled: true,
            l2_ttl: Duration::from_secs(parse_u64("cache.l2_ttl_seconds", 3600)?),
            warming_enabled: true,
            warming_interval: Duration::from_secs(parse_u64("cache.warming_interval_minutes", 30)? * 60),
            warm_scenarios: config_map.get("cache.warm_scenarios")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| vec![
                    "personalized_home".into(),
                    "continue_watching".into(),
                    "trending_now".into(),
                    "live_tv".into(),
                ]),
        };

        // Observability
        let observability = ObservabilityConfig {
            tracing_enabled: parse_bool("observability.tracing_enabled", true)?,
            metrics_enabled: parse_bool("observability.metrics_enabled", true)?,
            log_level: parse_val("logging.level", "info"),
            log_format: parse_val("logging.format", "text"),
            jaeger_endpoint: config_map.get("observability.jaeger_endpoint").cloned(),
            prometheus_endpoint: config_map.get("observability.prometheus_endpoint").cloned(),
            otlp_endpoint: parse_val("tracing.otlp_endpoint", "http://localhost:4318/v1/traces"),
            otlp_protocol: parse_val("tracing.otlp_protocol", "http"),
            sampling_rate: parse_f64("tracing.sampling_rate", 1.0)?,
            batch_size: parse_u32("tracing.batch_size", 512)? as usize,
            max_queue_size: parse_u32("tracing.max_queue_size", 2048)? as usize,
        };

        // Create a normalized map for case-insensitive lookup
        let normalized_map: HashMap<String, String> = config_map.iter()
            .map(|(k, v)| (k.to_lowercase(), v.clone()))
            .collect();

        let get_val = |key: &str, default: &str| {
            normalized_map.get(&key.to_lowercase())
                .cloned()
                .unwrap_or_else(|| default.to_string())
        };

        // Security
        let security = SecurityConfig {
            license_key: get_val("security.license_key", ""),
            license_server_url: get_val("security.license_server_url", ""),
            hardware_id_salt: get_val("security.hardware_id_salt", ""),
            anti_debug_enabled: parse_bool("security.anti_debug_enabled", true)?,
            binary_protection_enabled: parse_bool("security.binary_protection_enabled", true)?,
            license_validation_interval: parse_u64("security.license_validation_interval", 3600)?,
            mobile_api_key: get_val("security.mobile_api_key", ""),
            web_api_key: get_val("security.web_api_key", ""),
            tv_api_key: get_val("security.tv_api_key", ""),
            system_api_key: get_val("security.system_api_key", ""),
            jwt_secret_key: get_val("security.jwt_secret_key", ""),
            ..SecurityConfig::default()
        };

        // Experiments
        let experiments = ExperimentsConfig {
            enabled: parse_bool("experiments.enabled", false)?,
            assignment_method: match parse_val("experiments.assignment_method", "random").as_str() {
                "hash" => AssignmentMethod::Hash,
                "thompson_sampling" => AssignmentMethod::ThompsonSampling,
                _ => AssignmentMethod::Random,
            },
        };

        // Resilience
        let circuit_breaker = CircuitBreakerConfig {
            enabled: parse_bool("resilience.circuit_breaker.enabled", true)?,
            failure_rate_threshold: parse_f64("resilience.circuit_breaker.failure_rate_threshold", 0.5)?,
            slow_call_rate_threshold: parse_f64("resilience.circuit_breaker.slow_call_rate_threshold", 0.5)?,
            slow_call_duration: Duration::from_secs(parse_u64("resilience.circuit_breaker.slow_call_duration_secs", 2)?),
            minimum_calls: parse_u64("resilience.circuit_breaker.minimum_calls", 10)?,
            wait_duration_in_open_state: Some(Duration::from_secs(parse_u64("resilience.circuit_breaker.wait_duration_in_open_state_secs", 30)?)),
            permitted_calls_in_half_open_state: Some(parse_u64("resilience.circuit_breaker.permitted_calls_in_half_open_state", 3)?),
            sliding_window_size: parse_u64("resilience.circuit_breaker.sliding_window_size", 100)? as usize,
            sliding_window_type: match parse_val("resilience.circuit_breaker.sliding_window_type", "count").as_str() {
                "time" => SlidingWindowType::TimeBased,
                _ => SlidingWindowType::CountBased,
            },
            writable_stack_trace_enabled: parse_bool("resilience.circuit_breaker.writable_stack_trace_enabled", true)?,
            record_exceptions: vec![],
            ignore_exceptions: vec![],
            recovery_timeout: Duration::from_secs(60),
            half_open_max_calls: 10,
            call_timeout: Duration::from_secs(30),
            max_concurrent_calls: parse_u32("bulkhead.max_concurrent_calls", 100)? as usize,
            consecutive_failure_threshold: None,
            bulkhead_enabled: parse_bool("bulkhead.enabled", true)?,
            bulkhead_per_endpoint: parse_bool("bulkhead.per_endpoint", true)?,
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
            defaults: ResilienceDefaults {
                circuit_breaker: circuit_breaker.clone(),
                retry: RetryConfig {
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

        // Notifications
        let notifications = NotificationConfig {
            enabled: parse_bool("notifications.enabled", true)?,
            adaptor: match parse_val("notifications.adaptor", "polling").as_str() {
                "kafka" => NotificationAdaptorKind::Kafka,
                "resend" => NotificationAdaptorKind::Resend,
                _ => NotificationAdaptorKind::Polling,
            },
            resend: ResendConfig {
                api_key: parse_val("notifications.resend.api_key", ""),
                from_email: parse_val("notifications.resend.from_email", "noreply@bongas-ai.com"),
                from_name: parse_val("notifications.resend.from_name", "Bongas-AI"),
            },
        };

        // Search
        let search = SearchConfig {
            host: parse_val("search.host", "http://localhost:7700"),
            api_key: parse_val("search.api_key", ""),
            index_name: parse_val("search.index_name", "items"),
            timeout_ms: parse_u64("search.timeout_ms", 500)?,
            max_hits: parse_u32("search.max_hits", 100)? as usize,
            typo_tolerance: parse_bool("search.typo_tolerance", true)?,
        };

        Ok(AppConfig {
            server,
            database,
            redis,
            clickhouse,
            ingestion,
            security,
            ml,
            pipeline,
            cache,
            circuit_breaker,
            error,
            analytics,
            observability,
            resilience,
            resilience_metrics: ResilienceMetricsConfig::default(),
            experiments,
            hive_mind,
            notifications,
            search,
        })
    }

    /// Validate cross-module configuration dependencies.
    fn validate_app_config(config: &AppConfig) -> ConfigResult<()> {
        validate_config_fn(config)
            .map_err(|e: anyhow::Error| ConfigError::Validation(e.to_string()))
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
