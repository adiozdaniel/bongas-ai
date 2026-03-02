# 🖼️ Pages: UI Orchestration

> **Dynamic discovery and layout management for recommendation-driven UIs.**

The Pages module provides first-class support for dynamic UI orchestration. It enables the system to define and manage which recommendation scenarios appear on specific UI pages (e.g., Home, Movies, Music) and in what order.

---

## 🏗️ Architecture

- **`Types`**: Domain models for page slugs, layouts, and management requests.
- **`Manager`**: The central coordinator handling layout resolution, LRU caching, and runtime updates.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🏷️ Types**](./types/README.md) | Domain models and serialization logic for page layouts. |
| [**🕹️ Manager**](./manager/README.md) | High-performance orchestration and caching of UI definitions. |

---
[🏠 Back to Project Root](../../README.md)
