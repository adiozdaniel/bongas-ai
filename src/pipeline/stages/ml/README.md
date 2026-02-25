# 🧠 Pipeline Stages: ML

> **Deep inference for high-fidelity ranking and scoring.**

ML stages use specialized runtimes (ONNX, PyTorch) and vector databases for complex neural inference tasks.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| [**🤖 BERT4Rec**](./ml_inference_bert4rec/README.md) | Transformer-based sequential recommender for session-aware ranking. |
| [**🔮 ONNX Inference**](./onnx_inference/README.md) | General-purpose ranker using a neural network (XGBoost/LightGBM or MLP). |
| [**🔍 Similarity**](./onnx_inference_similarity/README.md) | Embedding-based similarity calculation for reranking. |
| [**📊 Meta Scorer**](./meta_scorer/README.md) | Blends multiple model outputs into a unified score. |
| [**🧩 Heuristic Aggregator**](./heuristic_aggregator/README.md) | Combines rule-based logic with ML outputs. |
| [**🏎️ Multi-Action Ranker**](./multi_action_ranker/README.md) | Optimizes for multiple objectives (Click + Watch + Share). |
| [**🗼 Two Tower**](./ml_inference_two_tower/README.md) | High-performance user/item retrieval and ranking. |
| [**📏 Inference Similarity**](./ml_inference_similarity/README.md) | Generic embedding-based scoring stage. |

---
[⬅️ Back to Stages Main](../README.md)
