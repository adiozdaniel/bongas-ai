# ⚙️ Runtime: Execution State & Context

The Runtime sub-module provides the lifecycle and execution context for the recommendation engine.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🧬 Context**](./context/README.md) | Shared request-scoped execution state. |
| [**🚀 Runtime**](./runtime/README.md) | Application bootstrap and long-running execution loops. |

---

## 🎯 Design Principles

- **Stateful Management**: Graceful start and shutdown orchestration.
- **Contextual Integrity**: Ensures visitor identity and device state are preserved throughout a discovery stream.
- **Lifecycle Awareness**: Engine-wide health and readiness signal management.

---

[🏠 Hub](../../../../docs/HUB.md) | [⚡ Runtime Main](../README.md)
