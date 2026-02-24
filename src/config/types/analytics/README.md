# 📉 Config Type: Analytics

Defines how the application collects and exports its internal telemetry data. Supports multiple formats and configurable export intervals.

---

## 🏗️ Analytics Pipeline

```mermaid
graph LR
    Coll[Collection Interval] --> Proc[Histogram Precision]
    Proc --> Export[Export Format]
    Export --> Dest[Export Path]
```

---
[⬅️ Back to Types Main](../README.md)
