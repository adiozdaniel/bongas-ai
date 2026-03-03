# 🏭 Factory: Discovery Pipeline Lifecycle

The Factory sub-module manages the dynamic lifecycle of recommendation pipelines—from initial compilation to scenario registry management.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🎬 Scenarios Manager**](./scenarios_manager/README.md) | Lifecycle management and governance for recommendation pipelines. |
| [**🏭 Scenario Factory**](./scenario_factory/README.md) | Pipeline compilation and assembly logic for dynamic scenarios. |

---

## 🎯 Design Principles

- **Lifecycle Governance**: Atomic reloads and state management for discovery strategies.
- **Dynamic Assembly**: Compiled pipeline logic from high-level scenario definitions.
- **Safety First**: Validates all scenario definitions before activation.
