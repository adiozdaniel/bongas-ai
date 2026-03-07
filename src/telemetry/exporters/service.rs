//! Telemetry exporters for various output targets.
//!
//! Provides exporters for stdout, stderr, file, and future OTLP support.

use std::io::{self, Write};

use opentelemetry::KeyValue;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    runtime,
    trace::{Sampler, TracerProvider},
    Resource,
};

// ─── Writer Trait ───────────────────────────────────────────────────────────

/// Trait for writers that can be used with tracing.
pub trait TelemetryWriter: Write + Send + Sync + 'static {}

impl<T: Write + Send + Sync + 'static> TelemetryWriter for T {}

// ─── Buffered Writer ────────────────────────────────────────────────────────

/// Buffered writer for improved performance.
pub struct BufferedWriter<W: Write> {
    inner: W,
    buffer: Vec<u8>,
    capacity: usize,
}

impl<W: Write> BufferedWriter<W> {
    /// Create a new buffered writer.
    pub fn new(inner: W, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn flush_internal(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            self.inner.write_all(&self.buffer)?;
            self.buffer.clear();
        }
        self.inner.flush()
    }
}

impl<W: Write> Write for BufferedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buffer.len() + buf.len() > self.capacity {
            self.flush_internal()?;
        }

        if buf.len() > self.capacity {
            self.inner.write_all(buf)?;
        } else {
            self.buffer.extend_from_slice(buf);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_internal()
    }
}

impl<W: Write> Drop for BufferedWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ─── OTLP Exporter (Live) ───────────────────────────────────────────────────

/// Supported OTLP transport protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OtlpProtocol {
    /// OTLP over gRPC (high performance).
    Grpc,
    /// OTLP over HTTP/JSON (better firewall compatibility).
    #[default]
    Http,
}

/// Configuration for the OTLP exporter.
#[derive(Debug, Clone)]
pub struct OtlpExporterConfig {
    /// OTLP endpoint URL.
    pub endpoint: String,
    /// The transport protocol to use.
    pub protocol: OtlpProtocol,
    /// Headers to include in requests (e.g., API keys).
    pub headers: Vec<(String, String)>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Batch size for exporting spans.
    pub batch_size: usize,
    /// Maximum queue size for the span processor.
    pub max_queue_size: usize,
    /// Sampling rate (0.0 to 1.0).
    pub sampling_rate: f64,
}

impl Default for OtlpExporterConfig {
    fn default() -> Self {
        Self {
            endpoint: String::from("http://localhost:4318/v1/traces"),
            protocol: OtlpProtocol::Http,
            headers: Vec::new(),
            timeout_ms: 10000,
            batch_size: 512,
            max_queue_size: 2048,
            sampling_rate: 1.0,
        }
    }
}

/// Initialize the OTLP tracing pipeline.
pub fn init_otlp_pipeline(
    config: &OtlpExporterConfig,
    service_name: String,
    environment: String,
) -> Result<TracerProvider, opentelemetry::trace::TraceError> {
    let endpoint = config.endpoint.clone();
    let timeout = std::time::Duration::from_millis(config.timeout_ms);

    let exporter = match config.protocol {
        OtlpProtocol::Http => {
            let mut http_builder = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .with_timeout(timeout);
            
            for (key, value) in &config.headers {
                http_builder = http_builder.with_headers(std::collections::HashMap::from([(key.clone(), value.clone())]));
            }
            http_builder.build()?
        }
        OtlpProtocol::Grpc => {
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .with_timeout(timeout)
                .build()?
        }
    };

    let resource = Resource::new_with_defaults(vec![
        KeyValue::new("service.name", service_name),
        KeyValue::new("deployment.environment", environment),
    ]);

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.sampling_rate))))
        .with_resource(resource)
        .build();

    Ok(provider)
}
