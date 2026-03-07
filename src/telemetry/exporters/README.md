# 📤 Telemetry: Exporters

> **Pluggable telemetry delivery targets.**

The Exporters module provides multiple delivery targets for structured logs and traces, ensuring visibility across diverse environments.

---

## 🚀 Supported Exporters

### 1. Standard Output / Error
Real-time streaming of structured JSON or human-readable logs to the console.

### 2. File-Based Storage
Persistent storage of telemetry data. Implements a thread-safe `FileWriter` that can be wired into the global tracing subscriber.

### 3. OTLP (OpenTelemetry Protocol)
Netflix-grade remote export for distributed tracing.

- **Dual Protocol Support**: Supports both `gRPC` (via Tonic) and `HTTP` (via Reqwest) transports.
- **Dynamic Header Injection**: Allows passing authentication or metadata headers to OTLP collectors.
- **Performance Batching**: Uses `BatchSpanProcessor` with configurable `batch_size` and `max_queue_size` to ensure zero impact on request latency.

---

## 🛠️ Configuration

Exporters are configured via the `TelemetryConfig` and `OtlpExporterConfig` structs, allowing for environment-specific tuning.

```rust
let config = OtlpExporterConfig {
    endpoint: "http://otel-collector:4317".to_string(),
    protocol: OtlpProtocol::Grpc,
    batch_size: 512,
    ..Default::default()
};
```

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Telemetry Main](../README.md)
