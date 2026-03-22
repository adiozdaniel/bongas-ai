# 🎯 THE STAGE: Inference Pillar

The Inference pillar is the latency-sensitive fast path of the Cortex. It handles real-time model execution and feature resolution.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🕯️ Candle**](./candle/README.md) | Pure-Rust inference engine using the Candle framework. |
| [**🔍 Embeddings**](./embeddings/README.md) | Vector resolution and high-speed similarity search. |
| [**💎 Features**](./features/README.md) | Low-latency feature retrieval from Feature Store. |

---

## 🎯 Design Principles

- **Zero-Dependency Architecture**: 100% Rust-native ML stack (No ONNX Runtime C++ libs).
- **Sub-ms Inference**: Highly optimized CPU-based execution for secure VPCs.
- **Resilient Execution**: Every inference call is protected by Netflix-grade circuit breakers.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to ML Training](../README.md)
