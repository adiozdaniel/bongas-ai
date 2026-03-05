# ☁️ Config Source: Spring Cloud

Integrates with a remote Spring Cloud Config Server to enable centralized configuration management across distributed microservices.

---

## 🏗️ Remote Flow

```mermaid
graph LR
    App[Bongas AI] -->|REST API| SCC[Spring Cloud Config Server]
    SCC -->|Fetch| Git[(Git Repository)]
    Git -->|Return| SCC
    SCC -->|JSON| App
```

---

## 🔑 Key Features

- **Dynamic Updates**: Supports fetching the latest configuration without application restarts.
- **Profile Support**: Automatically handles different environments (e.g., `prod`, `staging`, `dev`).
- **Labeling**: Enables versioning through Git branches or tags.

---

[🏠 Hub](../../../../docs/HUB.md) | [📡 Back to Sources Main](../README.md) | [🔝 Top](#️-config-source-spring-cloud)
