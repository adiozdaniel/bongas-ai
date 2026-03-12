# ⚡ Engine: Execution Manager

The high-performance core of the engine. It is responsible for the end-to-step execution of recommendation pipelines, including strategic path resolution and thundering herd protection.

---

## 🌊 Request Flow

```mermaid
sequenceDiagram
    participant API
    participant Exec as Execution Manager
    participant Strat as Strategy Resolver
    participant Stage as Staging Manager
    participant Pipe as Pipeline Factory

    API->>Exec: execute_scenario(ScenarioExecutionContext)
    Exec->>Strat: resolve_dynamic_path()
    Exec->>Stage: check_l2_cache()
    alt Cache Hit
        Stage-->>Exec: Cached Items
    else Cache Miss
        Exec->>Pipe: execute_linked_pipeline()
        Pipe-->>Exec: Scored Items
        Exec->>Stage: populate_cache()
    end
    Exec-->>API: RecommendationItems
```

---

[🏠 Hub](../../../../../docs/HUB.md) | [⚡ Core Main](../README.md) | [🔝 Top](#-engine-execution-manager)
