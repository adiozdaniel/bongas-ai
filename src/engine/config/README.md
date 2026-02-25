# ⚙️ Engine: Config

Defines the core dependencies required to bootstrap and run the `BongasEngine`. This module acts as the contract between the main application entry point and the engine runtime.

---

## 🏗️ Dependency Injection

```mermaid
graph LR
    App[Main Entry] -->|Inject| Deps[EngineDependencies]
    Deps -->|Bootstrap| Engine[BongasEngine]
    
    subgraph Dependencies
    Deps -.-> Config[AppConfig]
    Deps -.-> Pool[PgPool]
    Deps -.-> CB[CB Registry]
    Deps -.-> Metrics[Collector]
    end
```

---
[⬅️ Back to Engine Main](../README.md)
