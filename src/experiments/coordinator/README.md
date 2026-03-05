# 🎯 Experiment Coordinator

> **Orchestrating variant allocation and user participation.**

The Coordinator is responsible for determining which variant a user should see based on their ID, the experiment's traffic allocation, and persistence rules.

---

## 🛠️ Key Responsibilities

- **Variant Assignment**: Deterministic hashing of User IDs to ensure consistent variant assignment.
- **Traffic Control**: Managing the percentage of users enrolled in an experiment.
- **Participation Tracking**: Recording which users were exposed to which variant for downstream analytics.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Experiments Main](../README.md)
