# 🧠 Intelligence Pillar: The Engine's Brain

The Intelligence Pillar is the central coordination hub for all proactive and analytical logic in the Bongas-AI engine. It transforms raw data into actionable insights and personalized content strategies.

---

## 🏛️ Core Components

| Component | Responsibility |
| :--- | :--- |
| **SuggestionsManager** | Manages ML-generated strategy recommendations and human-in-the-loop approvals. |
| **HiveMindConnector** | Interface to the global HiveMind LLM for classification and vectorization. |
| **WorkersManager** | Orchestrates all background tasks (Clustering, Scaping, Maintenance). |
| **FatigueSynchronizer** | Pluggable tracker for item exposure state. |
| **AnalyticsSidecar** | High-performance telemetry buffer and ClickHouse exporter. |
| **StalenessEngine** | Real-time cache invalidation based on user engagement events. |

---

## 🎯 Design Philosophy

- **Proactive over Reactive**: The engine doesn't just wait for requests; background workers (The Pulse) constantly refine the data state.
- **Fail-Open Intelligence**: If an intelligence component fails, the engine gracefully falls back to reliable defaults (e.g., Global Trending).
- **Separation of Concerns**: Intelligence logic is decoupled from the request-handling "Stage" layer, communicating primarily through shared high-speed caches (Redis).

---

[🏠 Hub](../../../../docs/HUB.md) | [🏠 Intelligence Main](../../README.md) | [🔝 Top](#-intelligence-pillar-the-engines-brain)
