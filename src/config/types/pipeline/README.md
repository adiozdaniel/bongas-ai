# 🏗️ Config Type: Pipeline

Orchestration settings for the recommendation execution engine. Controls stage behavior, thundering herd protection, and pipeline-level circuit breaking.

---

## 🏗️ Engine Pipeline

```mermaid
graph LR
    Pipe[PipelineConfig] --> Stages[Stage Limits]
    Pipe --> Break[Circuit Breakers]
    Pipe --> Herd[Consolidation TTL]
```

---
[⬅️ Back to Types Main](../README.md)
