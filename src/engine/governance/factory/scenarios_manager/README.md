# 🎬 Engine: Scenarios Manager

Manages the lifecycle of recommendation scenarios. This includes loading pipeline definitions from the database, compiling them into executable forms, and enforcing governance limits on the number of active scenarios.

---

## 🔄 Hot-Reload Lifecycle

```mermaid
graph LR
    Trigger[API / Timer] --> Load[Load from DB]
    Load --> Gov{Governance Check}
    Gov -->|Truncate| Link[Link Pipelines]
    Link --> Atomic[Atomic Map Update]
    Atomic --> Final[Active Pipelines]
```

---
[⬅️ Back to Engine Main](../README.md)
