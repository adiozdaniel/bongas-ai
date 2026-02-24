use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub log_level: String,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_endpoint: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            metrics_enabled: true,
            log_level: "info".to_string(),
            jaeger_endpoint: None,
            prometheus_endpoint: None,
        }
    }
}
