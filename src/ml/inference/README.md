# 🎯 THE STAGE: Inference Pillar

The Inference pillar is the latency-sensitive fast path of the Cortex. It handles real-time model execution and feature resolution.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🤖 ONNX**](./onnx/README.md) | High-performance inference with ONNX Runtime. |
| [**🔍 Embeddings**](./embeddings/README.md) | Vector resolution and high-speed similarity search. |
| [**💎 Features**](./features/README.md) | Low-latency feature retrieval from Feature Store. |

---

## 🎯 Design Principles

- **Zero-Latency Target**: Optimized for sub-10ms model execution.
- **Hardware Aware**: Leverages CUDA/CoreML acceleration where available.
- **Stateless Inference**: Minimizes memory pressure through efficient buffer management.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to ML Training](../README.md)
