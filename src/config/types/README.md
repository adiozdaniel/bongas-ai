# 🧬 Configuration: Types

The domain models for application settings. This module contains strictly typed structs that mirror the application's hierarchical configuration structure.

---

## 🏗️ Type Hierarchy

```mermaid
graph TD
    App[AppConfig] --> S[ServerConfig]
    App --> D[DatabaseConfig]
    App --> R[ResilienceConfig]
    App --> I[IngestionConfig]
    App --> M[MlConfig]
    
    R --> CB[CircuitBreakerConfig]
    R --> E[ErrorConfig]
```

---

[🏠 Hub](../../../docs/HUB.md) | [⚙️ Back to Config Main](../README.md) | [🔝 Top](#-configuration-types)
