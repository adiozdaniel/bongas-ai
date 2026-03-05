# 📡 Telemetry: Observability Infrastructure

> **Centralized tracing, logging, and metrics for full-system visibility.**

The Telemetry module provides the foundation for observability in Bongas-AI. It orchestrates structured logging, distributed tracing, and high-performance metrics collection, ensuring that every request and system event is traceable from edge to core.

---

## 🏗️ Architecture

- **`Tracing`**: Global initialization of the tracing subscriber and layer stack.
- **`Config`**: Validated, builder-pattern driven configuration for all telemetry aspects.
- **`Middleware`**: Integration layers for Axum, DB, and Kafka span propagation.
- **`Context`**: Request ID generation and trace context propagation.
- **`Exporters`**: Pluggable writers for stdout, files, and remote OTLP collectors.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**⚙️ Config**](./config/README.md) | Ergonomic configuration and validation. |
| [**📑 Context**](./context/README.md) | Trace correlation and Request ID management. |
| [**📤 Exporters**](./exporters/README.md) | Pluggable delivery targets for telemetry data. |
| [**🛡️ Middleware**](./middleware/README.md) | Automatic instrumentation for service layers. |
| [**🔍 Tracing**](./tracing/README.md) | Core tracing lifecycle and initialization. |

---

[🏠 Hub](../../docs/HUB.md) | [🏠 Back to Project Root](../../README.md)
