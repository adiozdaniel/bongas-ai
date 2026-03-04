# 🔢 Pipeline Stages: Sort

> **Final presentation logic and result set trimming.**

Sort stages ensure the final list is correctly ordered, deduplicated, and sized for the requesting client.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| [**🔝 Sort By Score**](./sort_by_score/README.md) | The standard descending order sort based on calculated weights. |
| [**✂️ Deduplicate**](./deduplicate/README.md) | Removes duplicate item IDs across different fetch branches. |
| [**🛑 Limit**](./limit/README.md) | Caps the result set at a specific size (e.g., top 20). |
| [**📄 Paginate**](./paginate_results/README.md) | Handles offset and limit for multi-page requests. |
| [**🎯 Relevance**](./sort_by_relevance/README.md) | Secondary sort based on metadata-driven relevance scores. |

---
[⬅️ Back to Stages Main](../README.md)
