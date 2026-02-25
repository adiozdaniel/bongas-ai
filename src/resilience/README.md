# 🛡️ Resilience: Metrics & Observability

> **High-performance telemetry for the Composite Resilience Pattern.**

The Resilience module provides the metrics infrastructure that powers the system's self-healing capabilities. It collects, aggregates, and exports data from circuit breakers, bulkheads, and retries, enabling real-time observability and automated response to system degradation.

---

## 🏗️ Architecture

- **`Collector`**: High-throughput event processing for resilience signals.
- **`Registry`**: Central hub managing metrics for all resilience components.
- **`Histogram`**: HDR (High Dynamic Range) histograms for accurate latency tracking.
- **`Exporter`**: Pluggable export adapters for Prometheus, logs, or custom sinks.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**📥 Collector**](./collector/README.md) | In-memory aggregation of resilience events. |
| [**⚙️ Config**](./config/README.md) | Validation and configuration for the resilience stack. |
| [**📤 Exporter**](./exporter/README.md) | Metrics delivery to downstream observability platforms. |
| [**📊 Histogram**](./histogram/README.md) | HDR histograms for latency percentile calculation. |
| [**🗄️ Registry**](./registry/README.md) | Global registry for tracking all component metrics. |
| [**📄 Types**](./types/README.md) | Core data models and event definitions. |

---
[🏠 Back to Project Root](../../README.md)
