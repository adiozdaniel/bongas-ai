# 📡 Config Type: Observability

Global settings for tracing, logging, and metrics. Controls the verbosity and destination of system visibility data.

---

## 🏗️ Observability Stack

```mermaid
graph TD
    Obs[ObservabilityConfig] --> Log[Logging Level]
    Obs --> Trace[Tracing Samples]
    Obs --> Prom[Prometheus Scrape]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#-config-type-observability)
