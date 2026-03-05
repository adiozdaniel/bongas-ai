# 🧪 Experiments: A/B Testing & Bandits

> **Framework for safe, high-velocity experimentation and optimization.**

The Experiments module enables the recommendation engine to test new algorithms, UI variants, and business logic in production with minimal risk. It supports standard A/B/n testing and dynamic Multi-Armed Bandit (MAB) allocation.

---

## 🏗️ Architecture

- **`Coordinator`**: The central orchestrator for assigning users to experiment variants and tracking participation.
- **`Models`**: Data structures for defining experiments, variants, and allocation strategies.

---

## 🧩 Sub-Modules

| Module | Description |
| :--- | :--- |
| [**🎯 Coordinator**](./coordinator/README.md) | Orchestration logic for experiment lifecycles. |
| [**📊 Models**](./models/README.md) | Experiment and Variant data models. |

---

[🏠 Hub](../../docs/HUB.md)
