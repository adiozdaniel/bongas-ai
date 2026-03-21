# 🏗️ THE BACKSTAGE: Training Pillar

The Training pillar manages the heavy-lifting logic for model refinement, continuous learning, and heavy-compute workflows.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🏛️ Training Pillar**](./pillar/README.md) | High-level orchestration, state management, and weight swaps. |
| [**🕯️ Candle Engine**](./candle/README.md) | Native Rust training implementation and Student Head architectures. |
| [**📈 Online Learning**](./online/README.md) | Incremental model updates from real-time feedback loops. |

---

## 🎯 Design Principles

- **Isolated Compute**: Training workloads must never contend with the Inference fast-path.
- **Continuous Improvement**: Automated learning loops based on discovery metrics.
- **Asynchronous Processing**: Heavy-compute tasks are fully decoupled via worker queues.

---

[🏠 Hub](../../../docs/HUB.md) | [🏠 Main Training Documentation](../README.md)
