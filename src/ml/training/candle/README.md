# 🕯️ Candle: Native Rust ML Pillar

> **High-performance, zero-dependency on-premise learning.**

The Candle module provides the implementation of "Student Heads" and the native training loops using the [Candle](https://github.com/huggingface/candle) framework. This enables the Bongas-AI engine to adapt to local data within the client's VPC without requiring a Python runtime.

---

## 🏗️ Architecture

- **`architectures/`**: Definition of native model structures (MLPs, Transformers) in Rust.
- **`engine/`**: The core training logic (Backprop, Optimizers, Loss Functions).
- **`types/`**: Shared data models for training batches and samples.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🎨 Architectures**](./architectures/README.md) | Native Rust model architectures. |
| [**⚙️ Engine**](./engine/README.md) | Training services and optimization logic. |
| [**📦 Types**](./types/README.md) | Shared types and data models. |

---

[🏠 ML Root](../README.md) | [🏠 Back to Project Root](../../../../README.md)
