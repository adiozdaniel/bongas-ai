# 💓 Workers: Background Intelligence & Maintenance

The Workers sub-module manages the proactive background tasks and maintenance routines that keep the Bongas-AI engine optimized, resilient, and intelligent.

---

## 🏛️ Module Structure

All workers are orchestrated by a centralized manager that handles their lifecycle and resource allocation.

| Component | Responsibility | Location |
| :--- | :--- | :--- |
| **WorkersManager** | The heart of background orchestration. Starts and monitors all registered workers. | `manager.rs` |
| **TribeOrchestrator** | Periodically clusters user profiles into behavioral tribes based on embeddings. | `tribe_orchestrator/` |
| **RegionalPulseWorker** | Scrapes regional news and events for semantic ranking boosts. | `regional_pulse/` |
| **FatigueSynchronizer** | Pluggable state-synchronizer for item exposure tracking. | `fatigue_sync/` |

---

## 🎯 Design Principles

- **Unified Lifecycle**: All background tasks are registered with the `WorkersManager` and respond to the global engine shutdown signal.
- **Fail-Safe Operation**: Workers are designed to fail-open. If a background clustering task fails, the engine falls back to global trending rather than stalling.
- **Asynchronous Execution**: All workers run in dedicated tokio tasks to ensure they never interfere with the latency-critical request path.
- **Data Integrity**: Workers ensure strong synchronization between Postgres (System of Record) and Redis (High-Speed Access).

---

## 🚀 Adding a New Worker

To add a new intelligence worker:

1. Create a new directory under `src/engine/intelligence/workers/`.
2. Implement the worker logic with a `start` method that accepts a `broadcast::Receiver<()>`.
3. Register the worker in `WorkersManager` in `src/engine/intelligence/workers/manager.rs`.
4. Inject any new dependencies in `src/engine/coordination/builder.rs`.

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-workers-background-intelligence--maintenance)
