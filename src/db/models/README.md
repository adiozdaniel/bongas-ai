# 🧬 Database: Models

Internal data structures representing database entities. These models map directly to the PostgreSQL schema and are used by the repository layer for row mapping.

---

## 🏗️ Core Entities

| Model | Table | Description |
| :--- | :--- | :--- |
| **Scenario** | `scenarios` | Recommendation pipeline configuration. |
| **Feature** | `features` | ML feature definitions and metadata. |
| **User** | `users` | User profile and demographic data. |
| **Interaction** | `user_interactions` | Raw playback and reaction events. |

---
[⬅️ Back to Database Main](../README.md)
