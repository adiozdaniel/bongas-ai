# 🏗️ Dynamic Stages: Recommendation Building Blocks

> **The modular components for JSONB recommendation pipelines.**

The Dynamic Stages module provides a library of standardized recommendation components. Each stage can be dynamically plugged into a JSONB pipeline to transform or augment recommendation results.

---

## 🏗️ Architecture

- **`Collaborative Filtering`**: Retrieval based on user-item interaction similarities.
- **`Content Based`**: Retrieval based on item metadata and user preferences.
- **`Hybrid`**: Combined strategies for robust recommendation performance.
- **`Filters`**: Hard constraints (age, availability, safety).
- **`Boosters`**: Dynamic scoring adjustments for relevance and business goals.
- **`Diversifiers`**: Re-ranking to ensure variety and prevent filter bubbles.
- **`ONNX Stages`**: High-performance neural inference using ONNX models.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**📺 Collaborative Filtering**](./collaborative_filtering/README.md) | Item-to-item and user-to-user similarity matching. |
| [**🔍 Content Based**](./content_based/README.md) | Retrieval based on item metadata (genre, tags, language). |
| [**🧬 Hybrid**](./hybrid/README.md) | Blended retrieval strategies for cold-start and engagement. |
| [**🛡️ Filters**](./filters/README.md) | Enforcing safety, licensing, and availability constraints. |
| [**🚀 Boosters**](./boosters/README.md) | Adjusting scores based on freshness, popularity, and affinity. |
| [**🌈 Diversifiers**](./diversifiers/README.md) | Ensuring variety across genres and creators. |
| [**🧠 ONNX Stages**](./onnx_stages/README.md) | Neural ranking and scoring using ONNX-based models. |

---
[⬅️ Back to Dynamic Main](../README.md)
