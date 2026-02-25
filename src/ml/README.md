# 🧠 ML: Machine Learning Infrastructure

> **High-performance inference, feature management, and online learning.**

The ML module provides a resilient stack for deploying and managing machine learning models in production. It is designed to handle high-throughput inference requests while maintaining system stability through circuit breaking and graceful degradation.

---

## 🏗️ Architecture

The ML infrastructure is composed of several specialized layers that work together to deliver real-time recommendations.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🤖 ONNX Runtime**](./onnx_runtime/README.md) | The core inference engine with hardware acceleration. |
| [**📥 Model Loader**](./model_loader/README.md) | Resilient model fetching and hot-reloading logic. |
| [**💎 Feature Store**](./feature_store/README.md) | Real-time feature retrieval with low-latency caching. |
| [**🔍 Embeddings**](./embeddings/README.md) | High-performance embedding lookups and management. |
| [**🗄️ Model Registry**](./model_registry/README.md) | Versioned model management and shadow deployment. |
| [**🧵 Worker Queue**](./worker_queue/README.md) | Async task orchestration and backpressure management. |
| [**📈 Online Learning**](./online_learning/README.md) | Real-time model updates based on user feedback. |
| [**🏗️ Orchestrator**](./training_orchestrator/README.md) | Coordination of training and export workflows. |
| [**🛠️ Utils**](./utils/README.md) | Shared ML utilities and mathematical operations. |

---
[🏠 Back to Project Root](../../README.md)
