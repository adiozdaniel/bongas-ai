# 📥 Configuration: Loader

The `loader` sub-module is the entry point for the configuration system. It implements the Builder pattern to define layers and orchestrates the merging of multiple sources into a single, cohesive `AppConfig`.

---

## 🛠️ Loading Pipeline

```mermaid
sequenceDiagram
    participant App
    participant Loader
    participant Source
    participant Parser
    
    App->>Loader: new().with_source(X)
    App->>Loader: load()
    loop For each source
        Loader->>Source: load()
        Source-->>Loader: Flat HashMap
    end
    Loader->>Parser: parse_config_map()
    Parser-->>Loader: Typed AppConfig
    Loader->>Loader: validate()
    Loader-->>App: Result<Arc<AppConfig>>
```

---

[🏠 Hub](../../../docs/HUB.md) | [⚙️ Back to Config Main](../README.md) | [🔝 Top](#-configuration-loader)
