# 🔢 Pipeline Stages: Sort

> **Final presentation logic and result set trimming.**

Sort stages ensure the final list is correctly ordered, deduplicated, and sized for the requesting client.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| **Sort By Score** | The standard descending order sort based on calculated weights. |
| **Deduplicate** | Removes duplicate item IDs across different fetch branches. |
| **Limit** | Caps the result set at a specific size (e.g., top 20). |
| **Paginate** | Handles offset and limit for multi-page requests. |

---
[⬅️ Back to Stages Main](../README.md)
