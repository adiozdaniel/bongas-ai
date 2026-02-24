# 🏗️ Config Type: Resilience

Orchestrates the global defaults for the composite resilience pattern. Combines circuit breaker, retry, and timeout settings into a unified profile.

---

## 🏗️ Resilience Aggregation

```mermaid
graph LR
    Res[ResilienceConfig] --> CB[Circuit Breaker Defaults]
    Res --> RT[Retry Defaults]
    Res --> TO[Global Timeouts]
```

---
[⬅️ Back to Types Main](../README.md)
