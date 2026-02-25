use std::time::{Duration, Instant};

/// Contextual information attached to errors for tracing and debugging.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub request_id: Option<String>,
    pub timestamp: Instant,
    pub component: &'static str,
    pub instance: Option<&'static str>,
    pub operation: Option<&'static str>,
    pub metadata: Vec<(&'static str, String)>,
}

impl ErrorContext {
    pub fn new(component: &'static str) -> Self {
        Self {
            request_id: None,
            timestamp: Instant::now(),
            component,
            instance: None,
            operation: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_instance(mut self, instance: &'static str) -> Self {
        self.instance = Some(instance);
        self
    }

    pub fn with_operation(mut self, operation: &'static str) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_metadata(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.metadata.push((key, value.into()));
        self
    }

    pub fn elapsed(&self) -> Duration {
        self.timestamp.elapsed()
    }

    pub fn label(&self) -> String {
        match self.instance {
            Some(inst) => format!("{}_{}", self.component, inst),
            None => self.component.to_string(),
        }
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new("unknown")
    }
}
