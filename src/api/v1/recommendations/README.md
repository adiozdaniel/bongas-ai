# 🎯 API V1: Recommendation Symphony

The primary surface for delivering the Bongas-AI Symphony. This module orchestrates the **Server-Driven UI (SDUI)** flow through a unified streaming gateway.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [🎨 Smart Pages](../../../../docs/architecture/PAGES.md)

---

## 🏗️ The Genesis Flow

The client app enters the Symphony through a single root endpoint that resolves the entire application structure and initial content.

```mermaid
sequenceDiagram
    participant User
    participant Handler
    participant Orchestrator
    participant Brain
    
    User->>Handler: GET /api/v1/recommendation
    Handler->>Orchestrator: Resolve Landing Page & Nav Mesh
    Orchestrator->>User: Event: navigation (Instant-On)
    Orchestrator->>User: Event: sub_navigation (Personalized Hubs)
    Orchestrator->>User: Event: manifest (Landing Blueprint)
    
    par Parallel Execution
        Orchestrator->>Brain: Reorder Rows based on engagement
        Orchestrator->>Orchestrator: Early Safety Check (Maturity)
        Orchestrator->>Orchestrator: Execute Scenarios (Batch Concurrency: 5)
    end

    Orchestrator->>User: Event: row (Stream as finished)
    Orchestrator->>User: Event: continuation (Next Batch URL)
```

## 🔑 Key Symphony Features

- **The Genesis Gateway**: One root call to rule them all. No hardcoded home slugs.
- **Batch-Streaming**: Protects client memory by streaming rows in server-dictated batches.
- **Continuation Pattern**: Seamlessly handles deep-scrolling via `continuation` events.
- **Adaptive Resilience**: Integrated fallback logic and visitor-level rate limiting.

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-api-v1-recommendation-symphony)
