# 🥇 Pipeline: RANKING (The Pulse)

> **The intelligence domain — ML scoring, re-ranking, and diversification.**

Ranking stages represent the "Pulse" of the pipeline, where mathematical models and heuristic algorithms determine the final ordering of results. This domain is the "Heart" of Bongas-AI, utilizing advanced ONNX inference and complex re-ranking logic to ensure maximum user engagement and serendipity.

[🏠 Hub](../../../docs/HUB.md) | [🏗️ Architecture](../../../docs/architecture/SYMPHONY.md) | [📖 Pipeline Main](../README.md)

---

## 🏛️ Ranking Principles

- **Intelligence-First**: Heavy utilization of Machine Learning (BERT4Rec, Two-Tower) for precise item-user alignment.
- **Dynamic Re-ranking**: Heuristic boosters (Recency, Popularity) and diversification (MMR) provide the final behavioral "Pulse."
- **Performance Optimized**: Native ONNX runtime integration for high-throughput, low-latency inference.

---

## 🧩 Intelligence Manifest

| Stage | Type | Description |
| :--- | :--- | :--- |
| [**🤖 ONNX Inference**](./onnx_inference/README.md) | ML | Executes generic ONNX models for real-time scoring. |
| [**🛰️ BERT4Rec**](./ml_inference_bert4rec/README.md) | ML | Sequence-based transformer model for session-aware ranking. |
| [**🗼 Two-Tower**](./ml_inference_two_tower/README.md) | ML | High-scale retrieval and ranking using dual-embedding models. |
| [**📈 Boost Trending**](./boost_trending/README.md) | Heuristic | Dynamic weight adjustment for globally trending content. |
| [**👤 Personalization**](./boost_personalization/README.md) | Heuristic | Adjusts scores based on long-term user affinity vectors. |
| [**🔀 Diversify MMR**](./diversify_mmr/README.md) | Logic | Maximizes relevant diversity using Maximal Marginal Relevance. |
| [**🔝 Sort By Score**](./sort_by_score/README.md) | Logic | Final descending order sort based on all weights. |
| [**🛑 Limit**](./limit/README.md) | Logic | Caps the final result set for client-specific delivery. |
| [**🎯 Relevance**](./sort_by_relevance/README.md) | Logic | Secondary sort for fine-grained metadata alignment. |

---
[🏠 Hub](../../../docs/HUB.md) | [🔝 Top](#-pipeline-ranking-the-pulse)
