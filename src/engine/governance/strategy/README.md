# 🎬 Scenarios: Recommendation Strategies

> **Dynamic orchestration of recommendation logic through pluggable strategies.**

The Scenarios module defines the high-level strategies for delivering recommendations. It supports both static Rust-implemented scenarios and dynamic JSONB-defined pipelines stored in the database.

---

## 🏗️ Architecture

- **`Traits`**: Core abstractions for scenario execution and strategy management.
- **`Loader`**: High-performance retrieval of scenario configurations from persistent storage.
- **`Dynamic`**: A sophisticated execution engine for JSONB-based recommendation pipelines.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🎯 Traits**](./traits/README.md) | Standardized interfaces for all recommendation scenarios. |
| [**📥 Loader**](./loader/README.md) | Database-backed loading of scenario definitions. |
| [**🌈 Dynamic**](./dynamic/README.md) | JSONB pipeline execution and dynamic stage management. |

---
[🏠 Back to Project Root](../../README.md)
