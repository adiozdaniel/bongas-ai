# 🎬 API V1: Scenarios

Management endpoints for dynamic recommendation scenarios. Allows for hot-reloading pipeline logic and adjusting orchestration rules without service restarts.

---

## 🏗️ Scenario Lifecycle

```mermaid
graph LR
    Admin[Admin Client] -->|PUT / POST| API[Scenario API]
    API -->|Persist| DB[(PostgreSQL)]
    API -->|Trigger| Engine[Bongas Engine]
    Engine -->|Hot Reload| Pipeline[Active Pipeline]
```

---

## 🔑 Key Features

- **Hot Reload**: Instant application of changed pipeline definitions.
- **Bulk Operations**: Reload all scenarios or specific subsets via slugs.
- **Admin Security**: Protected by mandatory `X-Platform-Key` validation.

---
[⬅️ Back to V1 Main](../README.md)
