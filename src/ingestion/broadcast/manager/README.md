# 🏢 Ingestion: Manager

The central coordinator responsible for starting, monitoring, and shutting down all ingestion components. It ensures that data flows smoothly from sources to the processor.

---

## 🛠️ Management Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Initializing
    Initializing --> Starting : start()
    Starting --> Running : All sources healthy
    Running --> Degraded : Some sources failed
    Running --> Stopping : shutdown()
    Degraded --> Running : Recovery
    Stopping --> [*]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Broadcast Main](../README.md)
