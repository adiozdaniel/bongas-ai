# 📊 Experiment Models

> **Data structures for defining experimentation logic.**

This module contains the core models used to describe experiments, their variants, and the logic used to select between them.

---

## 🧩 Core Types

- **`Experiment`**: Defines the overall test, including ID, name, and total traffic allocation.
- **`Variant`**: A specific treatment (e.g., "Algorithm A") with its own weight.
- **`AllocationStrategy`**: Logic for distribution (e.g., Weighted, Multi-Armed Bandit).

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Experiments Main](../README.md)
