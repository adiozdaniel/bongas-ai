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
[⬅️ Back to Types Main](../README.md)
