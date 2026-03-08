# 🧬 Configuration: Types

The domain models for application settings. This module contains strictly typed structs that mirror the application's hierarchical configuration structure.

---

## 🏗️ Type Hierarchy

```mermaid
graph TD
    App[AppConfig] --> S[ServerConfig]
    App --> D[DatabaseConfig]
    App --> Redis[RedisConfig]
    App --> CH[ClickHouseConfig]
    App --> Sec[SecurityConfig]
    App --> I[IngestionConfig]
    App --> M[MlConfig]
    App --> P[PipelineConfig]
    App --> C[CacheConfig]
    App --> R[ResilienceConfig]
    App --> Obs[ObservabilityConfig]
    App --> Exp[ExperimentsConfig]
    App --> HM[HiveMindConfig]
    
    R --> CB[CircuitBreakerConfig]
    R --> E[ErrorConfig]
    R --> A[AnalyticsConfig]
```

---

[🏠 Hub](../../../docs/HUB.md) | [⚙️ Back to Config Main](../README.md) | [🔝 Top](#-configuration-types)
