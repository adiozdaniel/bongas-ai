# 🕯️ ML: Candle Inference Engine

> **The high-performance, pure-Rust inference engine.**

Replaces the legacy ONNX Runtime with a native Rust implementation via the **Candle** framework. By executing models directly in the core binary, we achieve a true single-binary architecture with ZERO external C++ dependencies, simplifying deployment into air-gapped client VPCs.

### 🌟 Key Features
- **Pure Rust Stack**: Compiled directly into the `bongas-ai` executable.
- **Safetensors Support**: High-performance model weight ingestion.
- **Netflix Resilience**: Guarded by Circuit Breakers, Bulkheads, and real-time Analytics.
- **Sub-ms Inference**: Optimized for CPU-based execution in secure environments.

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to ML Training](../../training/README.md)
