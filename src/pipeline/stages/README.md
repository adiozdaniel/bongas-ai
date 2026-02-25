# 🏗️ Pipeline: Stages

> **The modular building blocks of the Bongas-AI recommendation engine.**

This module contains the implementation of all pipeline stages, categorized by their functional role. Each stage is an isolated unit of logic that transforms or augments a stream of recommended items.

---

## 🧩 Stage Categories

| Category | Description |
| :--- | :--- |
| [**🚀 Boost**](./boost/README.md) | Scoring adjustments based on business logic, freshness, and affinities. |
| [**🌈 Diversify**](./diversify/README.md) | Re-ranking to ensure variety and prevent filter bubbles. |
| [**💎 Enrich**](./enrich/README.md) | Hydrating items with additional real-time metadata. |
| [**📥 Fetch**](./fetch/README.md) | Retrieval stages for various data sources and recommendation strategies. |
| [**🛡️ Filter**](./filter/README.md) | Hard-constraint exclusions (availability, language, content safety). |
| [**🧠 ML**](./ml/README.md) | Advanced inference stages using ONNX models and neural rankers. |
| [**🔢 Sort**](./sort/README.md) | Final ordering, pagination, and deduplication logic. |

---

## 🛠️ Stage Architecture

Each stage follows the **Pure Component** pattern:

- **`mod.rs`**: Clean export manifest.
- **`service.rs`**: Implementation of the `PipelineStage` trait.
- **`README.md`**: Documentation, parameters, and examples.

---
[⬅️ Back to Pipeline Main](../README.md)
