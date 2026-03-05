# 🏗️ THE BACKSTAGE: Training Pillar

The Training pillar manages the heavy-lifting logic for model refinement, continuous learning, and heavy-compute workflows.

---

## 🏛️ Sub-Modules

| Module | Description |
| :--- | :--- |
| [**📈 Online Learning**](./online/README.md) | Incremental model updates from real-time feedback loops. |
| [**🎼 Orchestration**](./orchestration/README.md) | Management of training and export pipelines. |
| [**🧵 Workers**](./workers/README.md) | Asynchronous heavy-compute worker queues. |

---

## 🎯 Design Principles

- **Isolated Compute**: Training workloads must never contend with the Inference fast-path.
- **Continuous Improvement**: Automated learning loops based on discovery metrics.
- **Asynchronous Processing**: Heavy-compute tasks are fully decoupled via worker queues.

---

[🏠 Hub](../../../docs/HUB.md) | [🏠 Main Training Documentation](../README.md)
