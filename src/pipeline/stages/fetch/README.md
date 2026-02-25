# 📥 Pipeline Stages: Fetch

> **Retrieval components for gathering candidate items.**

Fetch stages are usually the entry point of a pipeline or a branch. They interface with databases, caches, and ML services to retrieve an initial set of items.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| **User Preferences** | Fetches items based on explicit user profile interests. |
| **Similar Content** | Vector-search based retrieval for "More like this". |
| **New Releases** | Time-based retrieval for recently published items. |
| **Popular Content** | Global trending items from ClickHouse or Redis. |

---
[⬅️ Back to Stages Main](../README.md)
