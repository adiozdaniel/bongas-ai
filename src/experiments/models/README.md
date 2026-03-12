# 📊 Experiment Models

> **Data structures for defining experimentation logic.**

This module contains the core models used to describe experiments, their variants, and the logic used to select between them.

---

## 🧩 Core Types

- **`Experiment`**: Defines the overall test, including ID, name, variants, and assignment method.
- **`Variant`**: A specific treatment (e.g., "Algorithm A") with its own weight and configuration overrides.
- **`AssignmentMethod`**: Logic for distribution (e.g., Random, Hash, Thompson Sampling).
- **`Assignment`**: The result of a user assignment to an experiment.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Experiments Main](../README.md)
