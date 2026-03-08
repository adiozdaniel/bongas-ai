use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub log_level: String,
    pub log_format: String,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_endpoint: Option<String>,
    
    // OTLP Settings
    pub otlp_endpoint: String,
    pub otlp_protocol: String, // "grpc" or "http"
    pub sampling_rate: f64,
    pub batch_size: usize,
    pub max_queue_size: usize,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            metrics_enabled: true,
            log_level: "info".to_string(),
            log_format: "text".to_string(),
            jaeger_endpoint: None,
            prometheus_endpoint: None,
            otlp_endpoint: "http://localhost:4318/v1/traces".to_string(),
            otlp_protocol: "http".to_string(),
            sampling_rate: 1.0,
            batch_size: 512,
            max_queue_size: 2048,
        }
    }
}
