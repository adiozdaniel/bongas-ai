# ⚠️ Config Type: Error

Defines global error handling and recovery strategies. Controls backoff algorithms, retry limits, and error context preservation policies.

---

## 🏗️ Recovery Strategy

```mermaid
graph LR
    Err[ErrorConfig] --> Backoff[Backoff Strategy]
    Err --> Limits[Retry Counts]
    Err --> Context[Context Truncation]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#️-config-type-error)
